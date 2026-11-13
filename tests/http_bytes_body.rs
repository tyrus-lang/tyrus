//! A response whose body is bytes rather than text or a JSON object.
//!
//! Declared as a type in the route's `response` and in its `body`, like
//! `TextBody` and for the same reason: those two are the contract a reader
//! consults, and a route that quietly stopped producing or parsing JSON because
//! of a value's type or a header the sender chose would leave that contract
//! silent about what it answers with and what it accepts.

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
    "description": "http bytes body test"
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

/// Reads the raw response as bytes.
///
/// The point of this route is bodies that are not text, so nothing here may go
/// through a `String`: a lossy decode would turn the very bytes under test into
/// replacement characters and the assertions would pass on the wrong data.
fn get_raw(port: u16, path: &str) -> Option<Vec<u8>> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    socket
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut raw = Vec::new();
    let _ = socket.read_to_end(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

/// Splits a raw response into its lowercased headers and its body bytes.
fn split(response: &[u8]) -> (String, Vec<u8>) {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("a response has a header section");
    let headers = String::from_utf8_lossy(&response[..separator]).to_ascii_lowercase();
    (headers, response[separator + 4..].to_vec())
}

fn assert_json_body(body: &[u8], expected: serde_json::Value, message: &str) {
    let actual: serde_json::Value = serde_json::from_slice(body)
        .unwrap_or_else(|error| panic!("{message}: invalid JSON response: {error}"));
    assert_eq!(actual, expected, "{message}");
}

/// Bytes that are not text, an explicit content type, an empty body and a JSON
/// route on the same server, so the binary path cannot be passing by changing
/// the default for everyone.
#[test]
fn a_bytes_body_is_sent_unchanged_and_the_other_paths_are_untouched() {
    let root = project_root("http-bytes-body");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, StreamBody, TextBody }} from "tyrus:http";
import {{ ReadableStream }} from "tyrus:stream";

interface Doc {{
    readonly value: string;
}}

const sendStream = (reply: HttpReply<StreamBody>, source: ReadableStream<u8[]>): void => {{
    reply.status(HttpStatus.Ok).send(source);
}};

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/icon", {{
        response: {{ 200: BytesBody }},
        handler: (request, reply): void => {{
            // 0xFF is not valid UTF-8 and 0x00 ends a C string: a body that
            // survives both is a body no text representation is filtering.
            const pixels: u8[] = [0, 255, 10, 0, 66];
            reply.status(HttpStatus.Ok).send(pixels);
        }},
    }});
    app.get("/png", {{
        response: {{ 200: BytesBody }},
        handler: (request, reply): void => {{
            const bytes: u8[] = [137, 80, 78, 71];
            reply.type("image/png").status(HttpStatus.Ok).send(bytes);
        }},
    }});
    app.get("/empty", {{
        response: {{ 200: BytesBody }},
        handler: (request, reply): void => {{
            const nothing: u8[] = [];
            reply.status(HttpStatus.Ok).send(nothing);
        }},
    }});
    app.post("/stream", {{
        body: StreamBody,
        response: {{ 200: StreamBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send(request.body);
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
        Command::new(root.join("target/debug").join("http-bytes-body"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut icon = None;
    while Instant::now() < deadline {
        if let Some(response) = get_raw(port, "/icon") {
            icon = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let png = get_raw(port, "/png");
    let empty = get_raw(port, "/empty");
    let json = get_raw(port, "/json");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    let icon = icon.expect("the server never answered");
    let (headers, body) = split(&icon);
    assert!(
        headers.contains("content-type: application/octet-stream"),
        "a bytes body was not served as bytes:\n{headers}"
    );
    assert!(
        headers.contains("content-length: 5"),
        "the length does not describe the bytes:\n{headers}"
    );
    assert_eq!(
        body,
        vec![0u8, 255, 10, 0, 66],
        "the bytes did not survive the wire"
    );

    let (headers, body) = split(&png.expect("no response for the explicit content type"));
    assert!(
        headers.contains("content-type: image/png"),
        "`reply.type` did not override the binary default:\n{headers}"
    );
    assert_eq!(body, b"\x89PNG".to_vec());

    let (headers, body) = split(&empty.expect("no response for the empty body"));
    assert!(
        headers.contains("content-length: 0"),
        "an empty array did not produce an empty body:\n{headers}"
    );
    assert!(body.is_empty(), "an empty array produced {body:?}");

    let (headers, body) = split(&json.expect("no response for the JSON route"));
    assert!(
        headers.contains("content-type: application/json; charset=utf-8"),
        "the JSON path changed:\n{headers}"
    );
    assert_eq!(body, b"{\"value\": \"x\"}".to_vec());
}

/// The route's declared response decides, in both directions.
///
/// A `BytesBody` route sending an object is refused, and so is an object route
/// sending an array. Without the second half the feature would have made `send`
/// accept a `u8[]` anywhere, which is the opposite of what declaring the
/// response is for.
#[test]
fn the_declared_response_decides_what_a_bytes_send_accepts() {
    let root = project_root("http-bytes-body-mismatch");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions } from "tyrus:http";

interface Doc {
    readonly value: string;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/a", {
        response: { 200: BytesBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ value: "x" });
        },
    });
    app.get("/b", {
        response: { 200: Doc },
        handler: (request, reply): void => {
            const bytes: u8[] = [1, 2];
            reply.status(HttpStatus.Ok).send(bytes);
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
        stderr.contains("`reply.send` expects `u8[]`, found `object`"),
        "a bytes route accepted an object:\n{stderr}"
    );
    assert!(
        stderr.contains("`reply.send` expects `Doc`, found `u8[]`"),
        "an object route accepted an array:\n{stderr}"
    );
}

/// Posts a body and reads the raw response back.
fn post_raw(port: u16, path: &str, content_type: &str, body: &[u8]) -> Option<Vec<u8>> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(head.as_bytes()).ok()?;
    socket.write_all(body).ok()?;
    let mut raw = Vec::new();
    let _ = socket.read_to_end(&mut raw);
    if raw.is_empty() { None } else { Some(raw) }
}

/// A request body arrives as the bytes that were sent.
///
/// The round trip is the assertion that matters: a body that goes out equal to
/// what came in cannot have passed through a text decode, which is what the
/// `string` body would have done to `0xFF`. The `TextBody` route is here so the
/// two forms cannot be sharing one path, and the content type is deliberately
/// not JSON: a route reading a shapeless body has no format to refuse.
#[test]
fn a_request_body_arrives_as_the_bytes_that_were_sent() {
    let root = project_root("http-bytes-request");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TextBody }} from "tyrus:http";

interface Shape {{
    readonly size: i32;
    readonly first: i32;
    readonly last: i32;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/shape", {{
        body: BytesBody,
        response: {{ 200: Shape }},
        handler: (request, reply): void => {{
            const bytes: u8[] = request.body.copy();
            const size: i32 = bytes.length as i32;
            const first: i32 = size > 0 ? bytes[0] as i32 : -1;
            const last: i32 = size > 0 ? bytes[size - 1] as i32 : -1;
            reply.status(HttpStatus.Ok).send({{ size: size, first: first, last: last }});
        }},
    }});
    app.post("/mirror", {{
        body: BytesBody,
        response: {{ 200: BytesBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send(request.body.copy());
        }},
    }});
    app.post("/note", {{
        body: TextBody,
        response: {{ 200: TextBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send(`got: ${{request.body}}`);
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
        Command::new(root.join("target/debug").join("http-bytes-request"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));

    let blob: [u8; 5] = [0, 255, 10, 0, 66];
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut shape = None;
    while Instant::now() < deadline {
        if let Some(response) = post_raw(port, "/shape", "image/png", &blob) {
            shape = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mirrored = post_raw(port, "/mirror", "application/octet-stream", &blob);
    let empty = post_raw(port, "/shape", "application/octet-stream", &[]);
    let note = post_raw(port, "/note", "text/plain", b"hello");
    drop(server);
    let _ = fs::remove_dir_all(&root);

    let (_, body) = split(&shape.expect("the server never answered"));
    assert_json_body(
        &body,
        serde_json::json!({"first": 0, "last": 66, "size": 5}),
        "the body did not arrive as the bytes that were sent",
    );

    let (_, body) = split(&mirrored.expect("no response for the mirror route"));
    assert_eq!(
        body,
        blob.to_vec(),
        "a body that went out and came back is not the one that was sent"
    );

    let (_, body) = split(&empty.expect("no response for the empty body"));
    assert_json_body(
        &body,
        serde_json::json!({"first": -1, "last": -1, "size": 0}),
        "an empty body was not read as empty",
    );

    let (_, body) = split(&note.expect("no response for the text route"));
    assert_eq!(
        String::from_utf8_lossy(&body),
        "got: hello",
        "a `TextBody` request did not reach the handler as its text"
    );
}

/// `TextBody` and `BytesBody` describe a body, so a channel that is not one
/// refuses them.
///
/// Both have no fields, which is why this needs saying: every walk looking for
/// an unsupported field accepts an interface that has none, so `params:
/// TextBody` compiled and handed the handler an empty object.
#[test]
fn a_shapeless_body_is_refused_where_there_is_no_body() {
    let root = project_root("http-bytes-wrong-channel");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TextBody } from "tyrus:http";

interface Doc {
    readonly value: string;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/a/:id", {
        params: TextBody,
        response: { 200: Doc },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ value: "x" });
        },
    });
    app.get("/b", {
        query: BytesBody,
        response: { 200: Doc },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ value: "x" });
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

    assert!(!build.status.success(), "both routes should be refused");
    assert!(
        stderr.contains("`TextBody` describes a body, and `params` is not one"),
        "`params: TextBody` was accepted:\n{stderr}"
    );
    assert!(
        stderr.contains("`BytesBody` describes a body, and `query` is not one"),
        "`query: BytesBody` was accepted:\n{stderr}"
    );
    assert!(
        stderr.contains("E3293"),
        "the refusal has no diagnostic code:\n{stderr}"
    );
}

/// The body of one function in the emitted IR.
///
/// Matched on the line that *defines* the symbol, not on any line that mentions
/// it: `main` passes the callback's address to the route registration, so a
/// search for the name alone finds `main` first and asserts about the wrong
/// function entirely.
fn function_body<'a>(ir: &'a str, symbol: &str) -> Option<&'a str> {
    ir.split("\ndefine ").find(|body| {
        body.lines()
            .next()
            .is_some_and(|signature| signature.contains(symbol))
    })
}

/// An inline handler releases what its locals own.
///
/// Two leaks met here, one per request each, for the life of a server. The
/// request value the route wrapper builds is owned by nobody else, and nothing
/// released it: a `body` schema with a single `string` field lost its buffer on
/// every call. And an inline handler that ends without writing `return` skipped
/// the scope cleanup a named function performs in the same position, so every
/// `.copy()` in one leaked too.
///
/// Both are invisible to an assertion on output, which is why this test asserts
/// on the emitted IR instead: what the fixes add is a release, and a release
/// that stops being emitted is exactly the regression to catch. The bytes the
/// program prints would be identical either way.
#[test]
fn an_inline_handler_releases_what_it_owns() {
    let root = project_root("http-inline-release");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions } from "tyrus:http";

interface NameBody {
    readonly name: string;
}

interface Answer {
    size: i32;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.post("/name", {
        body: NameBody,
        response: { 200: Answer },
        handler: (request, reply): void => {
            const name: string = request.body.name.copy();
            reply.status(HttpStatus.Ok).send({ size: name.length as i32 });
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
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let ir = fs::read_to_string(root.join("target/debug").join("http-inline-release.ll")).unwrap();
    let _ = fs::remove_dir_all(&root);

    // The handler's own local: the copy is released before the implicit return.
    let handler = function_body(&ir, "@__tyrus_http_inline_handler_0")
        .expect("the inline handler was not emitted");
    assert!(
        handler.contains("tyrus_string_release"),
        "an inline handler emitted no release for its own local:\n{handler}"
    );

    // The request value the wrapper built for the handler. Asserted on the drop
    // *reading that slot*, which is the only thing that proves the release
    // happens: a bare `tyrus_string_release` in the wrapper proves nothing,
    // because JSON decoding releases its own temporaries either way, and the
    // slot alone proves nothing either, since it is allocated before the drop
    // is emitted.
    let wrapper = function_body(&ir, "@__tyrus_http_callback___tyrus_http_inline_handler_0")
        .expect("the callback wrapper was not emitted");
    assert!(
        wrapper
            .lines()
            .any(|line| line.contains("async.payload.drop") && line.contains("%http.request.owned")),
        "the route wrapper emitted no release for the request it built:\n{wrapper}"
    );
}
