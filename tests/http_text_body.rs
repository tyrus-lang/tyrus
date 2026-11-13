//! A response whose body is text rather than a JSON object.
//!
//! Declared as a type in the route's `response` rather than inferred from the
//! value's type: `response` is the contract a reader consults, and a `send`
//! that quietly stopped producing JSON because it was handed a string would
//! leave that contract silent about what the route answers with.

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
    "description": "http text body test"
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

fn get(port: u16, path: &str) -> Option<String> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    socket
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut raw = String::new();
    let _ = socket.read_to_string(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

/// Text, a text body with an explicit content type, and JSON on the same
/// server, so the text path cannot be passing by changing the default for
/// everyone.
#[test]
fn a_text_body_is_sent_as_text_and_json_is_untouched() {
    let root = project_root("http-text-body");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TextBody }} from "tyrus:http";

interface Doc {{
    readonly value: string;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/robots.txt", {{
        response: {{ 200: TextBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send("User-agent: *");
        }},
    }});
    app.get("/page", {{
        response: {{ 200: TextBody }},
        handler: (request, reply): void => {{
            reply.type("text/html; charset=utf-8").status(HttpStatus.Ok).send("<h1>hi</h1>");
        }},
    }});
    app.get("/json", {{
        response: {{ 200: Doc }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ value: "x" }});
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

    let server = ServerProcess(Some(
        Command::new(root.join("target/debug").join("http-text-body"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut text = None;
    while Instant::now() < deadline {
        if let Some(response) = get(port, "/robots.txt") {
            text = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let html = get(port, "/page");
    let json = get(port, "/json");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    let text = text.expect("the server never answered");
    assert!(
        text.to_ascii_lowercase()
            .contains("content-type: text/plain; charset=utf-8"),
        "a text body was not served as text:\n{text}"
    );
    assert!(
        text.ends_with("User-agent: *"),
        "the body was serialized rather than sent as it was written:\n{text}"
    );
    // The length has to describe the text, not a JSON rendering of it.
    assert!(
        text.to_ascii_lowercase().contains("content-length: 13"),
        "the body length does not describe the text:\n{text}"
    );

    let html = html.expect("no response for the explicit content type");
    assert!(
        html.to_ascii_lowercase()
            .contains("content-type: text/html; charset=utf-8"),
        "`reply.type` did not override the text default:\n{html}"
    );

    let json = json.expect("no response for the JSON route");
    assert!(
        json.to_ascii_lowercase()
            .contains("content-type: application/json; charset=utf-8")
            && json.ends_with("{\"value\": \"x\"}"),
        "the JSON path changed:\n{json}"
    );
}

/// The route's declared response decides, in both directions.
///
/// A `TextBody` route sending an object is refused, and so is an object route
/// sending text. Without the second half the feature would have made `send`
/// accept a string anywhere, which is the opposite of what declaring the
/// response is for.
#[test]
fn the_declared_response_decides_what_send_accepts() {
    let root = project_root("http-text-body-mismatch");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TextBody } from "tyrus:http";

interface Doc {
    readonly value: string;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/a", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ value: "x" });
        },
    });
    app.get("/b", {
        response: { 200: Doc },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("plain");
        },
    });
    try {
        app.listen(0, { host: "127.0.0.1" });
    } catch (error: HttpError) {
        console.error(`${error.code}: ${error.message}`);
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&build.stderr).into_owned();
    let _ = fs::remove_dir_all(&root);

    assert!(!build.status.success(), "both sends should be refused");
    assert!(
        stderr.contains("`reply.send` expects `string`, found `object`"),
        "a text route accepted an object:\n{stderr}"
    );
    assert!(
        stderr.contains("`reply.send` expects `Doc`, found `string`"),
        "an object route accepted text:\n{stderr}"
    );
}
