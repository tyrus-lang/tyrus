//! The bodies that are not ordinary text: empty, NUL-carrying, not UTF-8, and
//! never finished.
//!
//! Each of these is a place where a representation can quietly substitute
//! something for what arrived. A `string` that ends at a NUL truncates; a
//! decode that replaces invalid bytes destroys them; an empty body read as an
//! absent one reports the wrong error; and a transfer the client abandons can
//! leave a connection, a buffer, or a whole worker behind. The route in this
//! file answers with what it measured, so each of those substitutions would
//! show up as a different number rather than as nothing at all.

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
    "description": "http body edge test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

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
                    "SKIP: HTTP body testing requires permission to bind a local TCP listener ({error})."
                );
                return None;
            }
            Err(_) => {}
        }
    }
    panic!("no free port on the loopback interface");
}

struct ServerProcess(Option<std::process::Child>);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Sends a request whose body is written byte for byte, and returns the status
/// and the body.
fn post(port: u16, path: &str, content_type: &str, body: &[u8]) -> Option<(u16, String)> {
    let mut socket = match std::net::TcpStream::connect(("127.0.0.1", port)) {
        Ok(socket) => socket,
        Err(_) => return None,
    };
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(head.as_bytes()).ok()?;
    socket.write_all(body).ok()?;
    let mut raw = Vec::new();
    socket.read_to_end(&mut raw).ok()?;
    if raw.is_empty() {
        return None;
    }
    let separator = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&raw[..separator]).to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())?;
    Some((
        status,
        String::from_utf8_lossy(&raw[separator + 4..]).to_string(),
    ))
}

fn assert_json_response(
    response: Option<(u16, String)>,
    expected_status: u16,
    expected_body: serde_json::Value,
    message: &str,
) {
    let (status, body) = response.expect(message);
    assert_eq!(status, expected_status, "{message}");
    let body: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|error| panic!("{message}: invalid JSON response: {error}"));
    assert_eq!(body, expected_body, "{message}");
}

/// Announces a body and then abandons the connection without sending it.
fn abandon(port: u16, path: &str, announced: usize, sent: usize) {
    let Ok(mut socket) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
        return;
    };
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/octet-stream\r\nContent-Length: {announced}\r\nConnection: close\r\n\r\n"
    );
    let _ = socket.write_all(head.as_bytes());
    let _ = socket.write_all(&vec![b'x'; sent]);
    drop(socket);
}

const SOURCE: &str = r#"
import { BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, MultipartBody, MultipartPart, ServerOptions, TextBody } from "tyrus:http";

interface Measured {
    size: i32;
    first: i32;
}

interface NameBody {
    readonly name: string;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.post("/bytes", {
        body: BytesBody,
        response: { 200: Measured },
        handler: (request, reply): void => {
            const bytes: u8[] = request.body.copy();
            const size: i32 = bytes.length as i32;
            reply.status(HttpStatus.Ok).send({ size: size, first: size > 0 ? bytes[0] as i32 : -1 });
        },
    });
    app.post("/text", {
        body: TextBody,
        response: { 200: Measured },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ size: request.body.length as i32, first: -1 });
        },
    });
    app.post("/json", {
        body: NameBody,
        response: { 200: Measured },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ size: request.body.name.length as i32, first: -1 });
        },
    });
    app.post("/form", {
        body: MultipartBody,
        response: { 200: Measured },
        handler: (request, reply): void => {
            const parts: MultipartPart[] = request.body.copy();
            const size: i32 = parts.length as i32;
            reply.status(HttpStatus.Ok).send({ size: size, first: size > 0 ? parts[0].bytes.length as i32 : -1 });
        },
    });
    const server: HttpServer = app.listen(PORT, { host: "127.0.0.1" }) onfail {
        printError(`${server.code}: ${server.message}`);
    };
    print(`Server listening on port ${server.port()}`);
};
"#;

/// Every edge on one server, so none of them passes by being alone.
#[test]
fn a_body_survives_being_empty_binary_or_never_finished() {
    let root = project_root("http-body-edges");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        SOURCE.replace("PORT", &port.to_string()),
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

    let server = ServerProcess(Some(
        Command::new(root.join("target/debug").join("http-body-edges"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut empty_bytes = None;
    while Instant::now() < deadline {
        if let Some(response) = post(port, "/bytes", "application/octet-stream", b"") {
            empty_bytes = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let empty_text = post(port, "/text", "text/plain", b"");
    let empty_json = post(port, "/json", "application/json", b"");

    // A NUL is a byte like any other on the wire, and a representation that
    // ends at one would report four bytes here instead of six.
    let with_nul = post(port, "/bytes", "application/octet-stream", b"ab\0cdX");
    let text_with_nul = post(port, "/text", "text/plain", b"ab\0cd");

    // Not UTF-8 at all. The bytes route must hand them over unchanged; the text
    // route is text by definition and substitutes, which is why they are
    // different routes.
    let binary = post(port, "/bytes", "application/octet-stream", b"\xFF\xFE\x41");

    // A multipart body with no parts at all, and one whose single part is
    // empty: an absent part list and a present empty one are different answers.
    let no_parts = post(
        port,
        "/form",
        "multipart/form-data; boundary=B",
        b"--B--\r\n",
    );
    let empty_part = post(
        port,
        "/form",
        "multipart/form-data; boundary=B",
        b"--B\r\nContent-Disposition: form-data; name=\"a\"\r\n\r\n\r\n--B--\r\n",
    );

    // Transfers the client never finishes. The server must keep serving after
    // them, which is what the request afterwards checks.
    for _ in 0..20 {
        abandon(port, "/bytes", 100_000, 500);
    }
    let after_cancellations = post(port, "/bytes", "application/octet-stream", b"ok");

    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert_json_response(
        empty_bytes,
        200,
        serde_json::json!({"first": -1, "size": 0}),
        "an empty binary body was not read as empty",
    );
    assert_json_response(
        empty_text,
        200,
        serde_json::json!({"first": -1, "size": 0}),
        "an empty text body was not read as empty",
    );
    // An empty body is not a JSON document, so the route that declares one
    // refuses it rather than reading absent fields.
    assert_eq!(
        empty_json.expect("no answer for the empty JSON body").0,
        400,
        "an empty body satisfied a JSON schema"
    );

    assert_json_response(
        with_nul,
        200,
        serde_json::json!({"first": 97, "size": 6}),
        "a NUL truncated the body",
    );
    assert_json_response(
        text_with_nul,
        200,
        serde_json::json!({"first": -1, "size": 5}),
        "a NUL truncated the text",
    );
    assert_json_response(
        binary,
        200,
        serde_json::json!({"first": 255, "size": 3}),
        "bytes that are not text were altered on the way in",
    );

    assert_json_response(
        no_parts,
        200,
        serde_json::json!({"first": -1, "size": 0}),
        "a multipart body with no parts was not read as none",
    );
    assert_json_response(
        empty_part,
        200,
        serde_json::json!({"first": 0, "size": 1}),
        "a part with no bytes was not read as a part",
    );

    assert_json_response(
        after_cancellations,
        200,
        serde_json::json!({"first": 111, "size": 2}),
        "abandoned transfers left the server unable to serve",
    );
}
