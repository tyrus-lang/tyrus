//! A malformed request body must not be able to stop the server.
//!
//! The schema readers write each field into a slot of its own and hand the
//! value to the rejection path so it can be released. A read that fails writes
//! nothing, so that slot has to start empty: releasing an unwritten one frees
//! whatever the stack happened to hold, which crashed the process. It was
//! intermittent, which is why these tests send many requests rather than one.

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
    "description": "http malformed body test"
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

/// A server process that is killed however the test ends.
struct ServerProcess(Option<std::process::Child>);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn build_server(name: &str, port: u16, body: &str) -> (std::path::PathBuf, ServerProcess) {
    let root = project_root(name);
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

{body}

interface Ok {{
    readonly ok: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/user", {{
        body: CreateBody,
        response: {{ 200: Ok }},
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

    let server = Command::new(root.join("target/debug").join(name))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    (root, ServerProcess(Some(server)))
}

/// Sends one request and returns the raw response, or `None` if nothing came
/// back, which is what a dead server looks like from here.
fn post(port: u16, body: &str) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!(
        // The headers are a real client's rather than a minimal set. What the
        // unwritten slot held was whatever the last call left at that address,
        // and header text is a large part of that: the value the crash first
        // tried to free was a fragment of a `User-Agent`.
        "POST /user HTTP/1.1\r\nHost: localhost\r\nUser-Agent: curl/8.5.0\r\nAccept: */*\r\ncontent-type: application/json\r\ncontent-length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

/// Waits without sending a valid body.
///
/// A valid request first would leave the reader's stack holding the strings it
/// just read, and those are pointers a release survives. The crash needed the
/// slot to hold something that was never a pointer, which is what a request
/// that never reached the reader leaves behind.
fn wait_until_serving(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Ok(mut socket) = std::net::TcpStream::connect(("127.0.0.1", port)) {
            let _ = socket.set_read_timeout(Some(Duration::from_secs(5)));
            if socket
                .write_all(b"GET /user HTTP/1.1\r\nHost: localhost\r\nUser-Agent: curl/8.5.0\r\nAccept: */*\r\nConnection: close\r\n\r\n")
                .is_ok()
            {
                let mut raw = String::new();
                if socket.read_to_string(&mut raw).is_ok() && !raw.is_empty() {
                    return;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

/// Every malformed body is refused, and the server is still there afterwards.
fn refuses_without_dying(name: &str, schema: &str, bodies: &[&str]) {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server(name, port, schema);
    wait_until_serving(port);

    // The crash depended on what was left on the stack, so one request proved
    // nothing: it took a few before the released garbage was a pointer the
    // process could not free.
    for round in 0..24 {
        for body in bodies {
            let response = post(port, body);
            let Some(response) = response else {
                drop(server);
                let _ = fs::remove_dir_all(&root);
                panic!("the server stopped answering on round {round} after: {body}");
            };
            assert!(
                response.contains("400"),
                "expected a refusal for {body}, got:\n{response}"
            );
        }
        // Proof it is still serving, not merely still accepting.
        let healthy = post(port, r#"{"name":"ada","role":"admin","level":9}"#);
        let Some(healthy) = healthy else {
            drop(server);
            let _ = fs::remove_dir_all(&root);
            panic!("the server stopped serving valid requests on round {round}");
        };
        assert!(
            healthy.contains("200"),
            "a valid request failed after malformed ones on round {round}:\n{healthy}"
        );
    }

    drop(server);
    let _ = fs::remove_dir_all(&root);
}

/// A plain schema: no enums involved, which is where this was found.
#[test]
fn a_malformed_body_does_not_stop_a_server_with_plain_fields() {
    refuses_without_dying(
        "http-malformed-plain",
        "interface CreateBody {\n    readonly name: string;\n    readonly role: string;\n    readonly level: i32;\n}",
        &[
            r#"{"name":5,"role":"admin","level":9}"#,
            r#"{"name":"ada","role":"admin","level":"nine"}"#,
            r#"{"name":"ada","role":"admin"}"#,
            r#"{"name":"ada","role":5,"level":9}"#,
        ],
    );
}

/// The same for a schema whose fields are enums, where the reader also compares
/// the value it read against the declared members.
#[test]
fn a_malformed_body_does_not_stop_a_server_with_enum_fields() {
    refuses_without_dying(
        "http-malformed-enum",
        "enum Role {\n    Admin = \"admin\",\n    Member = \"member\",\n}\n\nenum Level {\n    Low = 1,\n    High = 9,\n}\n\ninterface CreateBody {\n    readonly name: string;\n    readonly role: Role;\n    readonly level: Level;\n}",
        &[
            r#"{"name":5,"role":"admin","level":9}"#,
            r#"{"name":"ada","role":"root","level":9}"#,
            r#"{"name":"ada","role":"admin","level":5}"#,
            r#"{"name":"ada","role":5,"level":9}"#,
            r#"{"name":"ada","role":"admin","level":"nine"}"#,
        ],
    );
}
