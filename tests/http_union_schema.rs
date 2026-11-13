//! Discriminated unions in a response schema.
//!
//! A discriminated union goes out as the object its live variant already is:
//! there is no envelope, and the discriminant field is what tells the reader
//! which variant it got. Reading one back is the other half of the contract and
//! is not implemented, so a request schema still refuses it, with a message
//! that says which half is missing rather than "unsupported".

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
    "description": "http union schema test"
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
                    "SKIP: loopback HTTP testing requires permission to bind a local TCP listener ({error})."
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

fn build_server(name: &str, port: u16) -> (std::path::PathBuf, ServerProcess) {
    let root = project_root(name);
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface JobResponse {{
    readonly id: i32;
    readonly state: {{ kind: "done"; value: i32 }} | {{ kind: "failed"; reason: string }};
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/job/done", {{
        response: {{ 200: JobResponse }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ id: 1, state: {{ kind: "done", value: 42 }} }});
        }},
    }});
    app.get("/job/failed", {{
        response: {{ 200: JobResponse }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ id: 2, state: {{ kind: "failed", reason: "timeout" }} }});
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

    let server = Command::new(root.join("target/debug").join(name))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    (root, ServerProcess(Some(server)))
}

fn get(port: u16, path: &str) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    socket.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

fn wait_until_serving(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(response) = get(port, "/job/done")
            && response.contains("200")
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

/// Each variant goes out as its own object, discriminant included.
#[test]
fn each_union_variant_is_written_as_the_object_it_is() {
    let Some(port) = test_port() else {
        return;
    };
    let (_root, _server) = build_server("http-union-schema", port);
    wait_until_serving(port);

    let done = get(port, "/job/done").expect("the server answered the first route");
    assert!(
        done.contains(r#""kind": "done""#) && done.contains(r#""value": 42"#),
        "the live variant did not carry its own fields:\n{done}"
    );
    assert!(
        !done.contains("reason"),
        "a field of the variant that is not live leaked into the response:\n{done}"
    );

    let failed = get(port, "/job/failed").expect("the server answered the second route");
    assert!(
        failed.contains(r#""kind": "failed""#) && failed.contains(r#""reason": "timeout""#),
        "the second variant did not carry its own fields:\n{failed}"
    );
    assert!(
        !failed.contains("value"),
        "a field of the variant that is not live leaked into the response:\n{failed}"
    );
}

/// A union crosses the wire both ways: written as the object its live variant
/// is, and read back by the discriminant choosing the variant.
#[test]
fn a_request_schema_reads_a_union_back_and_refuses_an_undeclared_variant() {
    let Some(port) = test_port() else {
        return;
    };
    let root = project_root("http-union-request");
    std::fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface JobBody {{
    readonly state: {{ kind: "done"; value: i32 }} | {{ kind: "failed"; reason: string }};
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/job", {{
        body: JobBody,
        response: {{ 200: JobBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ state: request.body.state }});
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
    let server = Command::new(root.join("target/debug").join("http-union-request"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let _guard = ServerProcess(Some(server));

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut ready = false;
    while Instant::now() < deadline {
        if let Some(response) = post_job(port, r#"{"state":{"kind":"done","value":42}}"#)
            && response.contains("200")
        {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(ready, "the server never answered on port {port}");

    let echoed =
        post_job(port, r#"{"state":{"kind":"failed","reason":"timeout"}}"#).expect("an answer");
    assert!(
        echoed.contains(r#""kind": "failed""#) && echoed.contains(r#""reason": "timeout""#),
        "the variant did not survive the round trip:\n{echoed}"
    );

    // A discriminant naming no declared variant is refused rather than guessed
    // at, which is the whole reason for writing it on the wire.
    let refused = post_job(port, r#"{"state":{"kind":"cancelled"}}"#).expect("an answer");
    assert!(
        refused.contains("400"),
        "an undeclared variant was accepted:\n{refused}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

fn post_job(port: u16, body: &str) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!(
        "POST /job HTTP/1.1\r\nHost: localhost\r\ncontent-type: application/json\r\ncontent-length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}
