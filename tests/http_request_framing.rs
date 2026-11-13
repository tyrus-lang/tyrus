//! How a request that cannot be framed is answered.
//!
//! A message carrying both `content-length` and `transfer-encoding` has two
//! possible ends, and every hop is free to believe a different one: that
//! disagreement is request smuggling. The layer beneath resolves it silently by
//! preferring the chunked framing, so the refusal has to be the server's own,
//! and only a real socket can show it.
//!
//! The same read also has to tell "too big" from "not readable". Answering
//! `413` to both told a client to send less when what it sent could not be read
//! at all.

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
    "description": "http request framing test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

/// A port the operating system confirmed is free.
///
/// These test binaries run in parallel, so a fixed or randomly guessed port
/// collides, and the failure is baffling: one test's server answers another
/// test's request, and the assertion reports a body it never asked for. That is
/// exactly how a port picked from the clock landed on a fixed `34_517` here.
/// Binding first and releasing narrows the window to the moment between this
/// call and the server's own bind.
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

struct Response {
    status: u16,
    body: String,
}

fn send_once(port: u16, request: &[u8]) -> Option<Response> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream.write_all(request).ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())?;
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default();
    Some(Response { status, body })
}

fn wait_until_serving(port: u16, request: &[u8]) -> Response {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(response) = send_once(port, request) {
            return response;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

fn post(headers: &str, body: &str) -> Vec<u8> {
    format!("POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\n{headers}Connection: close\r\n\r\n{body}").into_bytes()
}

#[test]
fn a_request_that_cannot_be_framed_is_refused_before_its_body_is_read() {
    let root = project_root("http-request-framing");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface Body {{
    readonly name: string;
}}

interface Echo {{
    readonly ok: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/echo", {{
        body: Body,
        response: {{ 200: Echo }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    try {{
        app.listen({port}, {{ host: "127.0.0.1" }});
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

    let mut server = Command::new(root.join("target/debug").join("http-request-framing"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let valid = wait_until_serving(port, &post("Content-Length: 14\r\n", r#"{"name":"ada"}"#));

    // Two framings at once: refused as the sender's error, not resolved by
    // guessing which one to believe.
    let ambiguous = send_once(
        port,
        &post(
            "Content-Length: 12\r\nTransfer-Encoding: chunked\r\n",
            r#"{"name":"x"}"#,
        ),
    );

    // Chunked framing that does not parse is unreadable, not oversized.
    let malformed = send_once(port, &post("Transfer-Encoding: chunked\r\n", "ZZZZ\r\n"));

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    assert_eq!(valid.status, 200, "body: {}", valid.body);
    assert_eq!(valid.body, r#"{"ok": true}"#);

    let ambiguous = ambiguous.expect("the ambiguous request was answered");
    assert_eq!(ambiguous.status, 400, "body: {}", ambiguous.body);
    assert!(
        ambiguous.body.contains("ambiguous_framing"),
        "unexpected body: {}",
        ambiguous.body
    );

    let malformed = malformed.expect("the malformed request was answered");
    assert_eq!(malformed.status, 400, "body: {}", malformed.body);
    assert!(
        malformed.body.contains("malformed_body"),
        "a framing failure must not be reported as an oversized body: {}",
        malformed.body
    );
}
