//! HTTPS with a certificate and key the program names.
//!
//! The point of these tests is that the encryption is real from the outside: a
//! TLS client completes a handshake against the certificate the program pointed
//! at, and a plaintext request to the same port gets nothing. A unit test on
//! the loading code could not tell those apart.

use rustls::pki_types::pem::PemObject;
use std::fs;
use std::io::{Read, Write};
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("tls")
        .join(name)
}

fn tyrus_string_literal(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn project_root(name: &str) -> std::path::PathBuf {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-{name}-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "http tls test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

/// A port the operating system confirmed is free. See the note in
/// `http_server_lifecycle.rs`.
/// A free port, and never the same one twice in this process.
///
/// Discovering a port by binding to 0 and closing the listener again lets the
/// kernel hand the same number to the next test that asks, and these tests ask in
/// parallel: whoever loses that race dies with the address already in use, and
/// its test then talks to the neighbour's server instead of its own, which is a
/// TLS test meeting a plaintext server or an `h2` test meeting one that never
/// advertised it. The counter never repeats inside the process, and the offset
/// taken from the process id keeps two test binaries that run at once from
/// starting at the same place.
fn test_port() -> Option<u16> {
    const FIRST: u16 = 20_000;
    const COUNT: u16 = 40_000;
    static NEXT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
    static START: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    let start = *START.get_or_init(|| (std::process::id() % u32::from(COUNT)) as u16);
    for _ in 0..COUNT {
        let offset = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let candidate = FIRST + (start.wrapping_add(offset) % COUNT);
        match std::net::TcpListener::bind(("127.0.0.1", candidate)) {
            Ok(listener) => {
                drop(listener);
                return Some(candidate);
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!(
                    "SKIP: loopback HTTP testing requires permission to bind a local TCP listener ({error})."
                );
                return None;
            }
            Err(_) => {}
        }
    }
    panic!("no free port on the loopback interface");
}

/// Builds and starts a server whose `listen` options are written by the caller.
fn build_server(name: &str, port: u16, options: &str) -> (std::path::PathBuf, ServerProcess) {
    let root = project_root(name);
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TlsOptions }} from "tyrus:http";

interface Health {{
    readonly ok: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    const server = app.listen({port}, {{ host: "127.0.0.1"{options} }}) onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
}};
"#
        ),
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let server = Command::new(root.join("target/debug").join(name))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    (root, ServerProcess::new(server))
}

/// A server process that is killed however the test ends.
///
/// A test that panics between spawning and killing leaves the server running
/// and holding its port. Nothing notices at the time: the failure lands on
/// whatever test runs next and finds an address in use, or worse, gets an
/// answer from a server it did not start.
struct ServerProcess(Option<std::process::Child>);

impl ServerProcess {
    fn new(child: std::process::Child) -> Self {
        Self(Some(child))
    }

    /// Waits for the process to end on its own and takes its output.
    fn into_output(mut self) -> std::process::Output {
        self.0
            .take()
            .expect("the child is taken exactly once")
            .wait_with_output()
            .unwrap()
    }

    /// Ends the process and takes whatever it printed.
    fn kill_and_take_output(mut self) -> std::process::Output {
        let mut child = self.0.take().expect("the child is taken exactly once");
        let _ = child.kill();
        child.wait_with_output().unwrap()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// A client that trusts exactly the test authority and nothing else.
///
/// Trusting only this one is what makes the test prove something: a client that
/// accepted any certificate would pass against a server presenting anything at
/// all, including one that was never configured.
fn tls_client_config() -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    let certificate = rustls::pki_types::CertificateDer::from_pem_file(fixture("ca.cert.pem"))
        .expect("the fixture authority is readable PEM");
    roots
        .add(certificate)
        .expect("the fixture certificate is a usable root");
    Arc::new(
        rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// Sends one HTTPS request and returns the raw response, or the handshake error.
fn https_get(port: u16) -> Result<String, String> {
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut connection = rustls::ClientConnection::new(tls_client_config(), server_name)
        .map_err(|error| error.to_string())?;
    let mut socket =
        std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|error| error.to_string())?;
    socket
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let mut stream = rustls::Stream::new(&mut connection, &mut socket);
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;
    let mut raw = Vec::new();
    // A server that closes the connection ends the read; `close_notify` missing
    // at the end is not a failure for a message that already arrived whole.
    let _ = stream.read_to_end(&mut raw);
    Ok(String::from_utf8_lossy(&raw).into_owned())
}

fn wait_until_serving_https(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(30);
    // The last failure is kept and reported: "never answered" on its own says
    // nothing about whether the server was absent, refused the handshake, or
    // answered something unexpected, and those are three different bugs.
    let mut last = String::from("no attempt was made");
    while Instant::now() < deadline {
        match https_get(port) {
            Ok(response) if response.contains("200") => return,
            Ok(response) => last = format!("answered without a 200:\n{response}"),
            Err(error) => last = error,
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("the HTTPS server never answered on port {port}: {last}");
}

/// Ends the server, removes its project and returns what it printed.
///
/// A failed assertion in these tests is usually explained by the server's own
/// output: a `listen` that found the address already in use, a certificate it
/// refused, a panic. That output used to be dropped with the process, so the
/// failure arrived with nothing to go on; every assertion below now carries it.
fn stop(root: std::path::PathBuf, server: ServerProcess) -> String {
    let output = server.kill_and_take_output();
    let _ = fs::remove_dir_all(&root);
    format!(
        "server stdout:\n{}server stderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn tls_options() -> String {
    format!(
        ", tls: {{ cert: {}, key: {} }}",
        tyrus_string_literal(&fixture("localhost.cert.pem").to_string_lossy()),
        tyrus_string_literal(&fixture("localhost.key.pem").to_string_lossy())
    )
}

#[test]
fn tyrus_string_literal_escapes_windows_paths_and_quotes() {
    assert_eq!(
        tyrus_string_literal(r#"C:\certs\server"key.pem"#),
        r#""C:\\certs\\server\"key.pem""#
    );
}

/// The server answers over TLS, and the certificate it presents is the one the
/// program named.
#[test]
fn a_server_with_a_certificate_serves_https() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-tls-serves", port, &tls_options());
    wait_until_serving_https(port);

    let response = https_get(port).expect("the handshake succeeds against the fixture root");
    let server_output = stop(root, server);

    assert!(
        response.contains("200"),
        "expected a 200 over TLS on port {port}, got:\n{response}\n{server_output}"
    );
    assert!(
        response.contains("\"ok\": true"),
        "expected the handler's body over TLS on port {port}, got:\n{response}\n{server_output}"
    );
}

/// A plaintext request to a TLS port is not served.
///
/// Without this the test above would still pass on a server that had quietly
/// ignored the `tls` option and was answering in the clear.
#[test]
fn a_plaintext_request_to_a_tls_port_is_not_served() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-tls-plaintext", port, &tls_options());
    wait_until_serving_https(port);

    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    socket
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    let server_output = stop(root, server);

    assert!(
        !raw.contains("200 OK"),
        "a plaintext request was served on TLS port {port}:\n{raw}\n{server_output}"
    );
}

/// A certificate path that names no file stops the server at `listen`.
///
/// Reported to whoever started the server rather than to whoever connects: the
/// mistake is in how the server was started, and a server that started anyway
/// would be one serving plaintext on a port meant to be encrypted.
#[test]
fn a_missing_certificate_is_reported_by_listen() {
    let Some(port) = test_port() else {
        return;
    };
    let options = format!(
        ", tls: {{ cert: {}, key: {} }}",
        tyrus_string_literal(&fixture("does-not-exist.cert.pem").to_string_lossy()),
        tyrus_string_literal(&fixture("localhost.key.pem").to_string_lossy())
    );
    let (root, server) = build_server("http-tls-missing-cert", port, &options);
    let stderr = String::from_utf8_lossy(&server.into_output().stderr).into_owned();
    let _ = fs::remove_dir_all(&root);

    assert!(
        stderr.contains("ERR_HTTP_TLS"),
        "expected a structured TLS error, got:\n{stderr}"
    );
    assert!(
        stderr.contains("certificate file does not exist"),
        "the error should name which file and why, got:\n{stderr}"
    );
}

/// A key that does not belong to the certificate is refused.
///
/// The pair is checked when the server starts rather than at the first
/// handshake, so the failure names the configuration instead of appearing as
/// every client being unable to connect.
#[test]
fn a_key_that_does_not_match_the_certificate_is_refused() {
    let Some(port) = test_port() else {
        return;
    };
    // A second key, kept in its own directory so it outlives the project the
    // server is built in, and generated here so the mismatch is real rather
    // than a file that merely fails to parse.
    let keys = project_root("http-tls-other-key");
    let other_key = keys.join("other.key.pem");
    let generated = Command::new("openssl")
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-keyout",
            other_key.to_str().unwrap(),
            "-out",
            keys.join("other.cert.pem").to_str().unwrap(),
            "-days",
            "1",
            "-nodes",
            "-subj",
            "/CN=other",
        ])
        .output();
    if !generated.is_ok_and(|output| output.status.success()) {
        // `openssl` is not part of the toolchain this project requires, so its
        // absence skips the case rather than failing it. The tests above do not
        // depend on it.
        let _ = fs::remove_dir_all(&keys);
        return;
    }

    let options = format!(
        ", tls: {{ cert: {}, key: {} }}",
        tyrus_string_literal(&fixture("localhost.cert.pem").to_string_lossy()),
        tyrus_string_literal(&other_key.to_string_lossy())
    );
    let (root, server) = build_server("http-tls-mismatch", port, &options);
    let stderr = String::from_utf8_lossy(&server.into_output().stderr).into_owned();
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&keys);

    assert!(
        stderr.contains("ERR_HTTP_TLS"),
        "expected a structured TLS error, got:\n{stderr}"
    );
    assert!(
        stderr.contains("does not match"),
        "the error should say the pair does not match, got:\n{stderr}"
    );
}

/// Sends one HTTP/2 `GET /health` over an already connected transport and
/// returns the first byte of the response header block and the body.
///
/// The frames are written by hand rather than through an HTTP/2 client so the
/// test depends on nothing but the protocol itself: the connection preface, an
/// empty SETTINGS frame, and one HEADERS frame whose block uses only the static
/// table and literals without Huffman coding.
fn h2_get<S: Read + Write>(
    stream: &mut S,
    scheme: &str,
    authority: &str,
) -> Result<(u8, Vec<u8>), String> {
    fn frame(kind: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
        let length = payload.len();
        let mut bytes = vec![
            (length >> 16) as u8,
            (length >> 8) as u8,
            length as u8,
            kind,
            flags,
        ];
        bytes.extend_from_slice(&stream_id.to_be_bytes());
        bytes.extend_from_slice(payload);
        bytes
    }
    let mut block = vec![0x82]; // :method GET
    block.push(if scheme == "https" { 0x87 } else { 0x86 }); // :scheme
    block.push(0x04); // :path, literal without indexing, indexed name
    block.push(b"/health".len() as u8);
    block.extend_from_slice(b"/health");
    block.push(0x01); // :authority, literal without indexing, indexed name
    block.push(authority.len() as u8);
    block.extend_from_slice(authority.as_bytes());

    let mut request = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n".to_vec();
    request.extend(frame(0x4, 0, 0, &[]));
    // END_STREAM | END_HEADERS on stream 1.
    request.extend(frame(0x1, 0x5, 1, &block));
    stream
        .write_all(&request)
        .map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut status = None;
    let mut body = Vec::new();
    loop {
        let mut header = [0u8; 9];
        stream
            .read_exact(&mut header)
            .map_err(|error| format!("reading a frame header: {error}"))?;
        let length =
            (usize::from(header[0]) << 16) | (usize::from(header[1]) << 8) | usize::from(header[2]);
        let (kind, flags) = (header[3], header[4]);
        let stream_id =
            u32::from_be_bytes([header[5], header[6], header[7], header[8]]) & 0x7fff_ffff;
        let mut payload = vec![0u8; length];
        stream
            .read_exact(&mut payload)
            .map_err(|error| format!("reading a frame payload: {error}"))?;
        match (kind, stream_id) {
            // GOAWAY: the server refused the connection.
            (0x7, 0) => return Err(format!("the server sent GOAWAY: {payload:?}")),
            // SETTINGS from the server: acknowledge it, as a client must.
            (0x4, 0) if flags & 0x1 == 0 => {
                stream
                    .write_all(&frame(0x4, 0x1, 0, &[]))
                    .map_err(|error| error.to_string())?;
            }
            (0x1, 1) => {
                let mut offset = 0;
                if flags & 0x8 != 0 {
                    offset += 1;
                }
                if flags & 0x20 != 0 {
                    offset += 5;
                }
                status = payload.get(offset).copied();
                if flags & 0x1 != 0 {
                    break;
                }
            }
            (0x0, 1) => {
                body.extend_from_slice(&payload);
                if flags & 0x1 != 0 {
                    break;
                }
            }
            (0x3, 1) => return Err(format!("the server reset the stream: {payload:?}")),
            _ => {}
        }
    }
    status
        .map(|status| (status, body))
        .ok_or_else(|| "the stream ended without a response header block".to_string())
}

/// A TLS client that offers `h2` is answered in HTTP/2.
///
/// The server advertises `h2` through ALPN, and this is the test that the
/// advertisement is kept: the negotiated protocol is `h2`, and a request sent
/// as HTTP/2 frames comes back as a `200` header block with the handler's body.
#[test]
fn a_tls_server_negotiates_http2_and_answers_in_frames() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-tls-h2", port, &tls_options());
    wait_until_serving_https(port);

    let mut config = (*tls_client_config()).clone();
    config.alpn_protocols = vec![b"h2".to_vec()];
    let server_name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
    let mut connection = rustls::ClientConnection::new(Arc::new(config), server_name).unwrap();
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let result = {
        let mut stream = rustls::Stream::new(&mut connection, &mut socket);
        h2_get(&mut stream, "https", &format!("localhost:{port}"))
    };
    let negotiated = connection.alpn_protocol().map(<[u8]>::to_vec);
    let server_output = stop(root, server);

    assert_eq!(
        negotiated.as_deref(),
        Some(&b"h2"[..]),
        "ALPN did not settle on h2 for port {port}\n{server_output}"
    );
    let (status, body) = result.unwrap_or_else(|error| {
        panic!("the HTTP/2 exchange over TLS on port {port} failed: {error}\n{server_output}")
    });
    // 0x88 is `:status 200` from the static table.
    assert_eq!(
        status, 0x88,
        "expected an indexed `:status 200` on port {port}\n{server_output}"
    );
    let body = String::from_utf8_lossy(&body);
    assert!(
        body.contains("\"ok\": true"),
        "expected the handler's body on port {port}, got:\n{body}\n{server_output}"
    );
}

/// A plaintext server answers HTTP/2 to a client that starts with the HTTP/2
/// preface (prior knowledge), on the same port that serves HTTP/1.1.
#[test]
fn a_plaintext_server_answers_http2_with_prior_knowledge() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-h2c", port, "");
    let deadline = Instant::now() + Duration::from_secs(30);
    let result = loop {
        let failure = match std::net::TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut socket) => {
                socket
                    .set_read_timeout(Some(Duration::from_secs(15)))
                    .unwrap();
                break h2_get(&mut socket, "http", &format!("127.0.0.1:{port}"));
            }
            Err(error) => error.to_string(),
        };
        if Instant::now() >= deadline {
            break Err(format!("the server never accepted a connection: {failure}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let server_output = stop(root, server);

    let (status, body) = result.unwrap_or_else(|error| {
        panic!("the HTTP/2 exchange on port {port} failed: {error}\n{server_output}")
    });
    assert_eq!(
        status, 0x88,
        "expected an indexed `:status 200` on port {port}\n{server_output}"
    );
    let body = String::from_utf8_lossy(&body);
    assert!(
        body.contains("\"ok\": true"),
        "expected the handler's body on port {port}, got:\n{body}\n{server_output}"
    );
}

/// `"development": { "https": true }` serves a server that names no `tls` over
/// HTTPS while `TYRUS_ENV=development`, with a certificate signed by a local
/// authority. A client that trusts only that authority is answered.
#[test]
fn development_https_serves_a_certificate_from_the_local_authority() {
    let Some(port) = test_port() else {
        return;
    };
    let root = project_root("http-dev-https");
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "http-dev-https",
    "version": "0.1.0",
    "authors": [],
    "description": "development https test"
  },
  "development": { "https": true },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus }} from "tyrus:http";

interface Health {{
    readonly ok: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    try {{
        app.listen({port});
    }} catch (error: HttpError) {{
        console.error(`${{error.code}}: ${{error.message}}`);
    }}
}};
"#
        ),
    )
    .unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let certificates = root.join("dev-https");
    let server = Command::new(root.join("target/debug").join("http-dev-https"))
        .env("TYRUS_ENV", "development")
        .env("TYRUS_DEV_HTTPS_DIR", &certificates)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    let authority = certificates.join("ca.cert.pem");
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last = String::from("the authority was never created");
    let mut answered = None;
    while Instant::now() < deadline {
        if authority.is_file() {
            let mut roots = rustls::RootCertStore::empty();
            match rustls::pki_types::CertificateDer::from_pem_file(&authority) {
                Ok(certificate) => {
                    roots.add(certificate).unwrap();
                    let config = Arc::new(
                        rustls::ClientConfig::builder()
                            .with_root_certificates(roots)
                            .with_no_client_auth(),
                    );
                    let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
                    let mut connection = rustls::ClientConnection::new(config, name).unwrap();
                    match std::net::TcpStream::connect(("127.0.0.1", port)) {
                        Ok(mut socket) => {
                            socket
                                .set_read_timeout(Some(Duration::from_secs(15)))
                                .unwrap();
                            let mut stream = rustls::Stream::new(&mut connection, &mut socket);
                            let sent = stream
                                .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                                .and_then(|()| stream.flush());
                            let mut raw = Vec::new();
                            let _ = stream.read_to_end(&mut raw);
                            let response = String::from_utf8_lossy(&raw).into_owned();
                            match sent {
                                Ok(()) if response.contains("200") => {
                                    answered = Some(response);
                                    break;
                                }
                                Ok(()) => last = format!("answered without a 200:\n{response}"),
                                Err(error) => last = error.to_string(),
                            }
                        }
                        Err(error) => last = error.to_string(),
                    }
                }
                Err(error) => last = format!("{error:?}"),
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    drop(server);
    let _ = fs::remove_dir_all(&root);
    let response = answered.unwrap_or_else(|| panic!("development HTTPS never answered: {last}"));
    assert!(response.contains("\"ok\": true"), "{response}");
}
