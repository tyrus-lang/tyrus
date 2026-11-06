//! Defences that belong to a connection, before any request exists.
//!
//! A slowloris client opens a connection and sends its request headers one byte
//! at a time, forever. It never becomes a request, so nothing a route or a
//! request timeout can see ever runs, and a few hundred such connections
//! exhaust a server that answers real traffic in milliseconds. The two limits
//! here are the answer, and both are on by default: a defence that has to be
//! opted into is one most programs will be without.

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
    "description": "http slowloris test"
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
/// `http_server_lifecycle.rs`: a guessed port makes one test answer another's
/// request, and the failure names a body nobody asked for.
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

fn build_server(name: &str, port: u16, options: &str) -> (std::path::PathBuf, std::process::Child) {
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
    (root, server)
}

fn wait_until_serving(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            if stream
                .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .is_ok()
            {
                let mut raw = String::new();
                if stream.read_to_string(&mut raw).is_ok() && raw.contains("200") {
                    return;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

/// A connection that never finishes its headers is closed on its own clock.
///
/// The request timeout cannot do this: it starts when a request exists, and the
/// whole point of the attack is that one never does.
#[test]
fn a_connection_that_never_finishes_its_headers_is_dropped() {
    let Some(port) = test_port() else {
        return;
    };
    // One second rather than the ten-second default, so the test measures the
    // clock instead of waiting out the shipping value.
    let (root, mut server) =
        build_server("http-slowloris-timeout", port, ", headerTimeout: \"1s\"");
    wait_until_serving(port);

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    // A request line and one header, and then nothing. The message is never
    // terminated, so it never becomes a request.
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\nX-Slow: a")
        .unwrap();
    stream.flush().unwrap();

    let started = Instant::now();
    let mut raw = Vec::new();
    let read = stream.read_to_end(&mut raw);
    let elapsed = started.elapsed();

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    assert!(
        read.is_ok(),
        "the connection was left open past its header clock: {read:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "the connection outlived its one-second header clock by far too much: {elapsed:?}"
    );
}

/// A header block larger than the cap is refused rather than buffered.
///
/// Without the cap a client can make the server hold megabytes per connection
/// before a single route has run, which is the same attack paid for in memory
/// instead of in time.
#[test]
fn a_header_block_over_the_cap_is_refused() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, mut server) = build_server(
        "http-slowloris-header-size",
        port,
        ", maxHeaderSize: \"8kib\"",
    );
    wait_until_serving(port);

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    // Well past the 8 KiB cap, written as complete headers so nothing else can
    // be blamed for the refusal.
    let padding = "x".repeat(1024);
    let mut wrote_all = true;
    for index in 0..64 {
        if stream
            .write_all(format!("X-Pad-{index}: {padding}\r\n").as_bytes())
            .is_err()
        {
            // The server closed on us mid-write, which is the refusal itself.
            wrote_all = false;
            break;
        }
    }
    let _ = stream.write_all(b"\r\n");
    let mut raw = String::new();
    let _ = stream.read_to_string(&mut raw);

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    // Either answer is the cap working: hyper replies `431` when it can, and
    // drops the connection when the client is still writing.
    assert!(
        !raw.contains("200 OK"),
        "a header block past the cap was served:\n{raw}"
    );
    assert!(
        !wrote_all || raw.contains("431") || raw.is_empty(),
        "expected a refusal for an oversized header block, got:\n{raw}"
    );
}
