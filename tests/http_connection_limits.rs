//! Limits that belong to the connection rather than to the request.
//!
//! A connection costs a file descriptor, a read buffer and a task from the
//! moment it is accepted, before it has sent anything a request limit could
//! count. `maxConcurrentRequests` cannot see any of that, which is why these
//! two settings exist separately from it.

use std::fs;
use std::io::{Read, Write};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
    "description": "http connection limit test"
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
/// its test then talks to the neighbour's server instead of its own. The counter
/// never repeats inside the process, and the offset taken from the process id
/// keeps two test binaries that run at once from starting at the same place.
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

fn build_server(name: &str, port: u16, options: &str) -> (std::path::PathBuf, ServerProcess) {
    let root = project_root(name);
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

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
        app.listen({port}, {{ host: "127.0.0.1"{options} }});
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

    let server = Command::new(root.join("target/debug").join(name))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    (root, ServerProcess::new(server))
}

/// Sends one request on a fresh connection and reads whatever comes back.
fn request_once(port: u16, read_timeout: Duration) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(read_timeout)).ok()?;
    socket
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

fn wait_until_serving(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(response) = request_once(port, Duration::from_secs(5))
            && response.contains("200")
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

fn stop(root: std::path::PathBuf, server: ServerProcess) {
    drop(server);
    let _ = fs::remove_dir_all(&root);
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
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Reads one response off a connection that stays open afterwards.
///
/// The `Connection: close` of `request_once` cannot be used here: the point is
/// to keep the connection.
fn read_one_response(socket: &mut std::net::TcpStream) -> String {
    let mut raw = Vec::new();
    let mut byte = [0u8; 1];
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        match socket.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                raw.push(byte[0]);
                // The body is one small JSON object and the response says how
                // long it is, so the closing brace is the end of the message.
                if raw.ends_with(b"}") && raw.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&raw).into_owned()
}

/// By default a connection carries more than one request.
#[test]
fn a_connection_is_reused_by_default() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-keep-alive-default", port, "");
    wait_until_serving(port);

    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    socket
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let first = read_one_response(&mut socket);
    // The same socket, a second time: this is what keep-alive means.
    let wrote = socket.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let second = read_one_response(&mut socket);
    stop(root, server);

    assert!(first.contains("200"), "the first request failed:\n{first}");
    assert!(
        wrote.is_ok() && second.contains("200"),
        "the connection was not reused for a second request:\n{second}"
    );
}

/// `keepAlive: false` closes the connection after its response.
///
/// For a server behind a proxy that pools its own connections, reuse on this
/// side buys nothing and costs a held socket.
#[test]
fn keep_alive_off_closes_the_connection_after_one_response() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-keep-alive-off", port, ", keepAlive: false");
    wait_until_serving(port);

    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    socket
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    // Reading to the end rather than to the closing brace: the server closing
    // is the behaviour under test, and it is what ends this read.
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    stop(root, server);

    assert!(raw.contains("200"), "the request was not served:\n{raw}");
    assert!(
        raw.to_ascii_lowercase().contains("connection: close"),
        "the response did not announce that the connection was closing:\n{raw}"
    );
}

/// A connection past `maxConnections` waits instead of being served.
///
/// The held connection has sent its request and been answered; keep-alive
/// leaves it open, and an open connection is what the limit counts. A request
/// limit cannot see it, because there is no request in flight.
#[test]
fn a_connection_past_the_limit_waits_for_a_slot() {
    let Some(port) = test_port() else {
        return;
    };
    // The header clock would otherwise close the held connection at its
    // ten-second default and hand the slot over for reasons other than the one
    // under test.
    let (root, server) = build_server(
        "http-max-connections",
        port,
        ", maxConnections: 1, headerTimeout: \"60s\"",
    );
    wait_until_serving(port);

    let mut held = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    held.set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    held.write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let first = read_one_response(&mut held);
    assert!(
        first.contains("200"),
        "the held connection was not served:\n{first}"
    );

    // The only slot is taken, so this one is not served: the kernel completes
    // the TCP handshake from the backlog, but the server never accepts it.
    let blocked = request_once(port, Duration::from_secs(2));
    assert!(
        blocked.is_none(),
        "a connection past the limit was served anyway:\n{blocked:?}"
    );

    // Releasing the slot lets the next connection through, which is what makes
    // this a limit rather than a permanent refusal.
    drop(held);
    let mut served = None;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Some(response) = request_once(port, Duration::from_secs(3)) {
            served = Some(response);
            break;
        }
    }
    stop(root, server);

    let served = served.expect("the freed slot was never handed to a waiting connection");
    assert!(
        served.contains("200"),
        "the connection that got the freed slot was not served:\n{served}"
    );
}
