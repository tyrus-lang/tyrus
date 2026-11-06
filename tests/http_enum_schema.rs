//! Enums in request and response schemas.
//!
//! An enum value is its declared value, and that is what crosses the wire: a
//! member declared `"admin"` is the JSON string `"admin"`, one declared `9` is
//! the number `9`. Nothing translates a number into its member name on the way
//! out, because then the value in the program and the value on the wire would
//! be two different things and only one of them would be what a `switch`
//! compares against.

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
    "description": "http enum schema test"
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

enum Role {{
    Admin = "admin",
    Member = "member",
}}

enum Level {{
    Low = 1,
    High = 9,
}}

interface CreateBody {{
    readonly name: string;
    readonly role: Role;
    readonly level: Level;
}}

interface UserResponse {{
    readonly name: string;
    readonly role: Role;
    readonly level: Level;
}}

interface User {{
    readonly id: i64;
    readonly name: string;
}}

interface ApiError {{
    readonly message: string;
}}

interface UsersResponse {{
    readonly users: User[];
}}

const listUsers = (request: {{}}, reply: HttpReply<User[] | ApiError>): void => {{
    reply.status(HttpStatus.Ok).send([]);
}};

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/user", {{
        body: CreateBody,
        response: {{ 200: UserResponse }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{
                name: request.body.name,
                role: request.body.role,
                level: request.body.level,
            }});
        }},
    }});
    app.get("/users", {{
        response: {{ 200: User[], 500: ApiError }},
        handler: listUsers,
    }});
    app.get("/health-text", {{
        response: {{ 200: string }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send("ok");
        }},
    }});
    app.get("/users-object", {{
        response: {{ 200: UsersResponse }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ users: [] }});
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

fn post(port: u16, body: &str) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let request = format!(
        "POST /user HTTP/1.1\r\nHost: localhost\r\ncontent-type: application/json\r\ncontent-length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    socket.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
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
        if let Some(response) = post(port, r#"{"name":"ada","role":"admin","level":9}"#)
            && response.contains("200")
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

/// An enum reaches the handler and comes back out with its declared value.
#[test]
fn an_enum_field_round_trips_as_its_declared_value() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-enum-round-trip", port);
    wait_until_serving(port);

    let response = post(port, r#"{"name":"ada","role":"member","level":1}"#)
        .expect("the server answers a well-formed body");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(response.contains("200"), "expected a 200, got:\n{response}");
    // The string enum keeps its text and the integer enum keeps its number:
    // neither is turned into the member's name on the way out.
    assert!(
        response.contains("\"role\": \"member\""),
        "the string enum did not come back as its declared value:\n{response}"
    );
    assert!(
        response.contains("\"level\": 1"),
        "the integer enum did not come back as its declared number:\n{response}"
    );
}

#[test]
fn an_empty_array_response_is_serialized_as_an_empty_json_array() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-empty-array-response", port);
    wait_until_serving(port);

    let response = get(port, "/users").expect("the server answers the users request");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(response.contains("200"), "expected a 200, got:\n{response}");
    assert!(
        response.ends_with("\r\n\r\n[]"),
        "an empty array response must be JSON `[]`, got:\n{response}"
    );
}

#[test]
fn a_primitive_response_schema_is_serialized_as_json() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-primitive-response", port);
    wait_until_serving(port);

    let response = get(port, "/health-text").expect("the server answers the health request");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(response.contains("200"), "expected a 200, got:\n{response}");
    assert!(
        response.ends_with("\r\n\r\n\"ok\""),
        "a string response must use JSON encoding, got:\n{response}"
    );
}

#[test]
fn an_empty_array_field_in_an_object_response_is_serialized_as_empty() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-empty-array-field-response", port);
    wait_until_serving(port);

    let response = get(port, "/users-object").expect("the server answers the users request");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(response.contains("200"), "expected a 200, got:\n{response}");
    assert!(
        response.ends_with("\r\n\r\n{\"users\": []}"),
        "an empty array field must be serialized as an empty JSON array, got:\n{response}"
    );
}

/// A value of the right type that is not a declared member is refused.
///
/// This is the check that makes an enum in a schema worth having. Without it
/// the wire could hand the program a `Level` holding `5`, which is no member
/// the program declared: every `switch` over it would fall through, and the
/// invariant the enum exists to state would have been broken by a stranger.
#[test]
fn a_value_outside_the_enum_is_refused() {
    let Some(port) = test_port() else {
        return;
    };
    let (root, server) = build_server("http-enum-outside", port);
    wait_until_serving(port);

    let unknown_text = post(port, r#"{"name":"ada","role":"root","level":9}"#)
        .expect("the server answers a rejected body");
    // A number of the right type, wrong value: nothing about the JSON is
    // malformed, and only the member list can refuse it.
    let unknown_number = post(port, r#"{"name":"ada","role":"admin","level":5}"#)
        .expect("the server answers a rejected body");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(
        unknown_text.contains("400"),
        "a string outside the enum was accepted:\n{unknown_text}"
    );
    assert!(
        unknown_number.contains("400"),
        "a number outside the enum was accepted:\n{unknown_number}"
    );
}
