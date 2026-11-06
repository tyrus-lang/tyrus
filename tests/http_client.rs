//! `fetch`, against a server this repository also compiled.
//!
//! Both ends are Tyrus programs on purpose: a client tested against a mock
//! proves the mock, and the pair proves that what one writes the other reads.
//!
//! A status is not a failure here. `404` arrives as an ordinary response with
//! `ok` false, because the request reached a server and got an answer;
//! collapsing that into the rejection a DNS failure produces would leave a
//! program unable to tell "the server said no" from "there was no server".

use std::fs;
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
    "description": "http client test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

/// A port for one fixture, distinct from every other port this binary hands out.
///
/// Binding port 0 and closing the listener answers a port that is free at that
/// instant and reserved by nobody, so two tests running in parallel here were
/// regularly handed the same number: each then started a fixture on it, one
/// won the bind, and the other test's client spoke to the wrong server. The
/// symptom was a transfer that failed under load and passed on its own, which
/// reads like a runtime bug and is not one.
///
/// A counter is what makes the numbers distinct within this process, and the
/// bind is what skips a number something outside it already holds.
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

fn build(root: &std::path::Path, name: &str, source: &str) {
    fs::write(root.join("src").join("main.tyr"), source).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "building `{name}` failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
}

/// Waits until the server answers, so the client does not race its start.
fn wait_until_serving(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never came up on port {port}");
}

const SERVER: &str = r#"
import { BytesBody, createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, StreamBody, TextBody } from "tyrus:http";
import { ReadableStream } from "tyrus:stream";

interface Echo {
    got: string;
    size: i32;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/hello", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("hello from tyrus");
        },
    });
    app.get("/blob", {
        response: { 200: BytesBody },
        handler: (request, reply): void => {
            // 0xFF is not valid UTF-8 and 0x00 ends a C string: bytes that
            // survive both survived the client too.
            const bytes: u8[] = [0, 255, 10];
            reply.status(HttpStatus.Ok).send(bytes);
        },
    });
    app.post("/echo", {
        body: TextBody,
        response: { 200: Echo },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ got: request.body, size: request.body.length as i32 });
        },
    });
    app.post("/relay", {
        body: StreamBody,
        response: { 200: StreamBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send(request.body);
        },
    });
    try {
        app.listen(PORT, { host: "127.0.0.1" });
    } catch (error: HttpError) {
        console.error(`${error.code}: ${error.message}`);
    }
};
"#;

const CLIENT: &str = r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError, HttpHeader } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const hello: HttpClientResponse = await fetch("http://127.0.0.1:PORT/hello");
        console.log(hello.status(), hello.ok());
        console.log(hello.text());
        // Asked in one case, sent in another: a header name is not case
        // sensitive, and the client must agree.
        const kind: string? = hello.header("Content-Type");
        if (kind) {
            console.log(kind);
        }
        const absent: string? = hello.header("x-not-sent");
        console.log(absent ? "present" : "absent");
        hello.close();

        const blob: HttpClientResponse = await fetch("http://127.0.0.1:PORT/blob");
        const raw: u8[] = blob.bytes();
        console.log(raw.length as i32, raw[0] as i32, raw[1] as i32);
        blob.close();

        const posted: HttpClientResponse = await fetch.post("http://127.0.0.1:PORT/echo", {
            headers: [{ name: "content-type", value: "text/plain" }],
            body: "sent by the client",
            timeout: "10s",
        });
        console.log(posted.status(), posted.text());
        posted.close();

        // A status the server chose, not a failure the transport had.
        const missing: HttpClientResponse = await fetch("http://127.0.0.1:PORT/nope");
        console.log(missing.status(), missing.ok());
        missing.close();
    } catch (error: HttpError) {
        console.log("unexpected", error.code, error.message);
    }
}
"#;

#[test]
fn fetch_reads_what_a_tyrus_server_wrote() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("http-client-server");
    build(
        &server_root,
        "server",
        &SERVER.replace("PORT", &port.to_string()),
    );
    let client_root = project_root("http-client");
    build(
        &client_root,
        "client",
        &CLIENT.replace("PORT", &port.to_string()),
    );

    let server = ServerProcess(Some(
        Command::new(server_root.join("target/debug").join("http-client-server"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    wait_until_serving(port);

    let output = Command::new(client_root.join("target/debug").join("http-client"))
        .output()
        .unwrap();
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    let _ = fs::remove_dir_all(&client_root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "200 true\n",
            "hello from tyrus\n",
            "text/plain; charset=utf-8\n",
            "absent\n",
            "3 0 255\n",
            "200 {\"got\": \"sent by the client\",\"size\": 18}\n",
            "404 false\n",
        )
    );
}

/// Streaming resolves once the response head exists, then yields the body
/// through the ordinary byte-stream interface instead of first collecting it.
#[test]
fn fetch_stream_exposes_the_response_body_as_a_readable_stream() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("http-client-stream-server");
    build(
        &server_root,
        "server",
        &SERVER.replace("PORT", &port.to_string()),
    );
    let client_root = project_root("http-client-stream");
    build(
        &client_root,
        "client",
        &format!(
            r#"
import {{ fetch, HttpClientStreamResponse, HttpError }} from "tyrus:http";
import {{ ReadableStream, WritableStream }} from "tyrus:stream";

async function main(): Promise<void> {{
    try {{
        const response: HttpClientStreamResponse = await fetch.stream("http://127.0.0.1:{port}/hello");
        console.log(response.status(), response.ok());
        const body: ReadableStream<u8[]> = await response.stream();
        // The body owns the transfer after `stream()` resolves, so releasing
        // the response head must not abort its still-readable body.
        response.close();
        const first: u8[]? = await body.read();
        if (first) {{
            console.log(first.length as i32);
        }}
        const end: u8[]? = await body.read();
        console.log(end ? "more" : "end");
    }} catch (error: HttpError) {{
        console.log("unexpected", error.code);
    }}
}}
"#
        ),
    );
    let server = ServerProcess(Some(
        Command::new(
            server_root
                .join("target/debug")
                .join("http-client-stream-server"),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap(),
    ));
    wait_until_serving(port);

    let output = Command::new(client_root.join("target/debug").join("http-client-stream"))
        .output()
        .unwrap();
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    let _ = fs::remove_dir_all(&client_root);

    assert!(
        output.status.success(),
        "the streamed client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "200 true\n16\nend\n"
    );
}

/// A streamed upload is read from its source only as the socket accepts it,
/// and can be relayed by a streaming server without materializing the body.
#[test]
fn fetch_sends_a_readable_stream_body_without_buffering_it() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("http-client-upload-server");
    build(
        &server_root,
        "server",
        &SERVER.replace("PORT", &port.to_string()),
    );
    let client_root = project_root("http-client-upload");
    fs::write(client_root.join("upload-source.bin"), [0_u8, 255, 10]).unwrap();
    build(
        &client_root,
        "client",
        &format!(
            r#"
import {{ fetch, FetchOptions, HttpClientResponse, HttpError }} from "tyrus:http";
import {{ FsError, readFileStream }} from "tyrus:fs";
import {{ ReadableStream }} from "tyrus:stream";

async function main(): Promise<void> {{
    const source: ReadableStream<u8[]> = readFileStream("upload-source.bin") onfail {{
        console.log(source.code);
        return;
    }};
    const result = await upload(source);
    match (result) {{
        Ok() => {{}}
        Err(error) => {{ console.log(error.code); }}
    }}
}}

async function upload(source: ReadableStream<u8[]>): Promise<Result<void, HttpError>> {{
    const response: HttpClientResponse = await fetch.post("http://127.0.0.1:{port}/relay", {{ bodyStream: source }});
    const body: u8[] = response.bytes();
    console.log(body.length as i32, body[0] as i32, body[1] as i32, body[2] as i32);
    response.close();
    return Ok();
}}
"#
        ),
    );
    let server = ServerProcess(Some(
        Command::new(
            server_root
                .join("target/debug")
                .join("http-client-upload-server"),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap(),
    ));
    wait_until_serving(port);

    let output = Command::new(client_root.join("target/debug").join("http-client-upload"))
        .current_dir(&client_root)
        .output()
        .unwrap();
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    let _ = fs::remove_dir_all(&client_root);

    assert!(
        output.status.success(),
        "the streamed upload client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3 0 255 10\n");
}

/// A generated client copies a chunked response to a file through the stream
/// API. The client never calls `bytes()` or `text()`, so a large transfer has
/// to keep its bounded stream contract until the destination closes.
#[test]
fn generated_client_copies_a_progressive_http_response() {
    let Some(origin_port) = test_port() else {
        return;
    };
    let mut origin = Command::new("python3")
        .arg("-c")
        .arg(PROGRESSIVE_ORIGIN.replace("ORIGIN_PORT", &origin_port.to_string()))
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::{BufRead, BufReader};
        let stdout = origin.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "ready", "the origin did not start");
    }

    let client_root = project_root("http-stream-copy-client");
    build(
        &client_root,
        "client",
        &format!(
            r#"
import {{ fetch, HttpClientStreamResponse, HttpError }} from "tyrus:http";
import {{ FsError, writeFileStream }} from "tyrus:fs";
import {{ ReadableStream, StreamError, WritableStream }} from "tyrus:stream";

async function transfer(response: HttpClientStreamResponse): Promise<Result<void, FsError>> throws StreamError {{
    const body: ReadableStream<u8[]> = await response.stream();
    const destination: WritableStream<u8[]> = writeFileStream("proxy-output.bin")?;
    await body.pipeTo(destination);
    await destination.close();
    response.close();
    return Ok();
}}

async function main(): Promise<void> {{
    const response: HttpClientStreamResponse = await fetch.stream("http://127.0.0.1:{origin_port}/source") onfail {{
        console.log("unexpected", response.code);
        return;
    }};
    try {{
        const result = await transfer(response);
        match (result) {{
            Ok() => {{ console.log("copied"); }}
            Err(error) => {{ console.log("unexpected", error.code); }}
        }}
    }} catch (error: StreamError) {{
        console.log("unexpected", error.code);
    }}
}}
"#
        ),
    );
    let output = Command::new(
        client_root
            .join("target/debug")
            .join("http-stream-copy-client"),
    )
    .current_dir(&client_root)
    .output()
    .unwrap();
    let _ = origin.kill();
    let _ = origin.wait();

    assert!(
        output.status.success(),
        "the streamed copy client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "copied\n");
    let copied = fs::read(client_root.join("proxy-output.bin")).unwrap();
    assert_eq!(copied.len(), 262144);
    assert_eq!(&copied[..131072], vec![1_u8; 131072].as_slice());
    assert_eq!(&copied[131072..], vec![2_u8; 131072].as_slice());
    let _ = fs::remove_dir_all(&client_root);
}

const PROGRESSIVE_ORIGIN: &str = r#"
import http.server, threading, time

# The port was chosen by binding a listener and closing it, so it is free at
# that instant and not reserved. Another process can take it before this one
# binds, and the retry is what keeps that from failing the test outright.
def bind_http(address, handler):
    deadline = time.time() + 10
    while True:
        try:
            return http.server.HTTPServer(address, handler)
        except OSError:
            if time.time() >= deadline:
                raise
            time.sleep(0.05)

class Handler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/source":
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/octet-stream")
        self.send_header("Content-Length", str(262144))
        self.end_headers()
        for value in (1, 2):
            data = bytes([value]) * 131072
            self.wfile.write(data)
            self.wfile.flush()
            time.sleep(0.05)
    def log_message(self, *args): pass

server = bind_http(("127.0.0.1", ORIGIN_PORT), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
print("ready", flush=True)
threading.Event().wait()
"#;

/// A streamed upload is one-shot, so a redirect is returned instead of trying
/// to replay a body whose producer has already been consumed.
#[test]
fn a_streamed_upload_preserves_the_redirect_response() {
    let Some(port) = test_port() else {
        return;
    };
    let mut server = Command::new("python3")
        .arg("-c")
        .arg(REDIRECT_UPLOAD_SERVER.replace("PORT", &port.to_string()))
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::{BufRead, BufReader};
        let stdout = server.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "ready", "the redirect server did not start");
    }

    let root = project_root("http-client-stream-redirect");
    fs::write(root.join("upload-source.bin"), [1_u8, 2, 3]).unwrap();
    build(
        &root,
        "client",
        &format!(
            r#"
import {{ fetch, FetchOptions, HttpClientResponse, HttpError }} from "tyrus:http";
import {{ FsError, readFileStream }} from "tyrus:fs";
import {{ ReadableStream }} from "tyrus:stream";

async function main(): Promise<void> {{
    const source: ReadableStream<u8[]> = readFileStream("upload-source.bin") onfail {{
        console.log("unexpected", source.code);
        return;
    }};
    const result = await upload(source);
    match (result) {{
        Ok() => {{}}
        Err(error) => {{ console.log("unexpected", error.code); }}
    }}
}}

async function upload(source: ReadableStream<u8[]>): Promise<Result<void, HttpError>> {{
    const response: HttpClientResponse = await fetch.post("http://127.0.0.1:{port}/upload", {{ bodyStream: source }});
    console.log(response.status(), response.header("location") ?? "missing");
    response.close();
    return Ok();
}}
"#
        ),
    );
    let output = Command::new(
        root.join("target/debug")
            .join("http-client-stream-redirect"),
    )
    .current_dir(&root)
    .output()
    .unwrap();
    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "the streamed redirect client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "307 /landing\n");
}

const REDIRECT_UPLOAD_SERVER: &str = r#"
import http.server
import threading
import time

# The port was chosen by binding a listener and closing it, so it is free at
# that instant and not reserved. Another process can take it before this one
# binds, and the retry is what keeps that from failing the test outright.
def bind_http(address, handler):
    deadline = time.time() + 10
    while True:
        try:
            return http.server.HTTPServer(address, handler)
        except OSError:
            if time.time() >= deadline:
                raise
            time.sleep(0.05)

class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        if self.path == "/upload":
            self.send_response(307)
            self.send_header("Location", "/landing")
            self.end_headers()
        else:
            self.send_response(500)
            self.end_headers()
    def log_message(self, *args): pass

server = bind_http(("127.0.0.1", PORT), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
print("ready", flush=True)
threading.Event().wait()
"#;

/// A request that never reaches a server rejects, and says why.
///
/// The code is what the caller acts on: a connection refused is not a name that
/// does not resolve, and neither is a clock that ran out.
#[test]
fn a_request_that_reaches_nobody_rejects_with_a_code() {
    let root = project_root("http-client-unreachable");
    // Port 1 on the loopback: nothing listens there, and nothing may without
    // privileges, so the connection is refused rather than answered.
    build(
        &root,
        "client",
        r#"
import { fetch, HttpClientResponse, HttpError } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const response: HttpClientResponse = await fetch("http://127.0.0.1:1/nothing");
        console.log("answered", response.status());
        response.close();
    } catch (error: HttpError) {
        console.log(error.code);
    }
}
"#,
    );
    let output = Command::new(root.join("target/debug").join("http-client-unreachable"))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ERR_HTTP_FETCH_CONNECT\n"
    );
}

/// A redirect to another origin does not carry the credentials.
///
/// This is the property nobody notices breaking: the request still succeeds,
/// the body still arrives, and the only difference is that a bearer token was
/// handed to a host the caller never addressed. It is pinned here because it is
/// inherited from the transport rather than written in this repository, and an
/// inherited guarantee is one a dependency bump can quietly remove.
///
/// The same-origin half is what keeps the rule from being over-broad: stripping
/// on every redirect would break the ordinary case of a server sending a client
/// from one of its own paths to another.
#[test]
fn a_redirect_to_another_origin_drops_the_credentials() {
    let Some(first) = test_port() else {
        return;
    };
    let Some(second) = test_port() else {
        return;
    };
    let mut origins = Command::new("python3")
        .arg("-c")
        .arg(
            ORIGINS
                .replace("FIRST_PORT", &first.to_string())
                .replace("SECOND_PORT", &second.to_string()),
        )
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    // The script prints one line once both listeners are bound.
    {
        use std::io::{BufRead, BufReader};
        let stdout = origins.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "ready", "the origins did not start");
    }

    let root = project_root("http-client-redirect");
    build(
        &root,
        "client",
        &r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError, HttpHeader } from "tyrus:http";

async function probe(url: string): Promise<void> throws HttpError {
    const answer: HttpClientResponse = await fetch(url, {
        headers: [
            { name: "authorization", value: "Bearer secret" },
            { name: "cookie", value: "session=secret" },
        ],
    });
    console.log(answer.text());
    answer.close();
}

async function main(): Promise<void> {
    try {
        await probe("http://127.0.0.1:PORT_A/same");
        await probe("http://127.0.0.1:PORT_A/cross");
        // `redirects: 0` follows none, so the redirect itself is the answer.
        const direct: HttpClientResponse = await fetch("http://127.0.0.1:PORT_A/cross", { redirects: 0 });
        console.log(direct.status());
        direct.close();
    } catch (error: HttpError) {
        console.log("unexpected", error.code);
    }
}
"#
        .replace("PORT_A", &first.to_string()),
    );
    let output = Command::new(root.join("target/debug").join("http-client-redirect"))
        .output()
        .unwrap();
    let _ = origins.kill();
    let _ = origins.wait();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "authorization=Bearer secret|cookie=session=secret\n",
            "authorization=<absent>|cookie=<absent>\n",
            "302\n",
        )
    );
}

/// Two origins on the loopback: one redirects, both echo what they received.
///
/// A different port is a different origin, which is all this needs to be.
const ORIGINS: &str = r#"
import http.server, threading, time

# The port was chosen by binding a listener and closing it, so it is free at
# that instant and not reserved. Another process can take it before this one
# binds, and the retry is what keeps that from failing the test outright.
def bind_http(address, handler):
    deadline = time.time() + 10
    while True:
        try:
            return http.server.HTTPServer(address, handler)
        except OSError:
            if time.time() >= deadline:
                raise
            time.sleep(0.05)

PORT_A, PORT_B = FIRST_PORT, SECOND_PORT

def echo(handler):
    auth = handler.headers.get("Authorization", "<absent>")
    cookie = handler.headers.get("Cookie", "<absent>")
    body = f"authorization={auth}|cookie={cookie}".encode()
    handler.send_response(200)
    handler.send_header("Content-Type", "text/plain")
    handler.send_header("Content-Length", str(len(body)))
    handler.end_headers()
    handler.wfile.write(body)

class A(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/cross":
            self.send_response(302)
            self.send_header("Location", f"http://127.0.0.1:{PORT_B}/landing")
            self.end_headers()
        elif self.path == "/same":
            self.send_response(302)
            self.send_header("Location", f"http://127.0.0.1:{PORT_A}/landing")
            self.end_headers()
        else:
            echo(self)
    def log_message(self, *a): pass

class B(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        echo(self)
    def log_message(self, *a): pass

for port, handler in ((PORT_A, A), (PORT_B, B)):
    srv = bind_http(("127.0.0.1", port), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
print("ready", flush=True)
threading.Event().wait()
"#;

/// The verbs hang off `fetch` itself.
///
/// Not exported on their own: `import { get, post }` would put those two names
/// in the scope of every file that wanted one, and `get` is a name a program
/// has its own uses for. Written on `fetch`, they cost no name at all.
///
/// The verb decides the method, so writing `method` in the options of one is
/// refused rather than resolved by a precedence rule: two places saying what a
/// request is would eventually disagree, and the reader would have to know
/// which one wins.
#[test]
fn the_verbs_send_what_they_name() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("http-verbs-server");
    build(
        &server_root,
        "server",
        &SERVER.replace("PORT", &port.to_string()),
    );
    let client_root = project_root("http-verbs");
    build(
        &client_root,
        "client",
        &r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError, HttpHeader } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const got: HttpClientResponse = await fetch.get("http://127.0.0.1:PORT/hello");
        console.log(got.status(), got.text());
        got.close();

        const posted: HttpClientResponse = await fetch.post("http://127.0.0.1:PORT/echo", {
            headers: [{ name: "content-type", value: "text/plain" }],
            body: "by the verb",
        });
        console.log(posted.status(), posted.text());
        posted.close();

        // A verb with no options at all still sends its method, which `HEAD`
        // proves by answering with the headers and no body.
        const head: HttpClientResponse = await fetch.head("http://127.0.0.1:PORT/hello");
        console.log(head.status(), head.text().length as i32);
        head.close();
    } catch (error: HttpError) {
        console.log("unexpected", error.code);
    }
}
"#
        .replace("PORT", &port.to_string()),
    );

    let server = ServerProcess(Some(
        Command::new(server_root.join("target/debug").join("http-verbs-server"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    wait_until_serving(port);
    let output = Command::new(client_root.join("target/debug").join("http-verbs"))
        .output()
        .unwrap();
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    let _ = fs::remove_dir_all(&client_root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "200 hello from tyrus\n",
            "200 {\"got\": \"by the verb\",\"size\": 11}\n",
            "200 0\n",
        )
    );
}

/// `method` is not an option at all: the verb is what names it.
///
/// Refused in both forms, because a reader arriving from the web `fetch` will
/// write it in either. The generic "expected `FetchOptions`, found `object`"
/// that a removed field otherwise produces is suppressed, since it says nothing
/// the reader can act on once the specific message named the fix.
#[test]
fn a_method_option_is_refused_and_says_what_to_write() {
    let root = project_root("http-verbs-method");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const verb: HttpClientResponse = await fetch.post("http://example.com", { method: "GET" });
        verb.close();
        const plain: HttpClientResponse = await fetch("http://example.com", { method: "POST" });
        plain.close();
    } catch (error: HttpError) {
        console.log(error.code);
    }
}
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

    assert!(!build.status.success(), "both calls should be refused");
    assert_eq!(
        stderr.matches("E3295").count(),
        2,
        "both forms should be refused, once each:\n{stderr}"
    );
    assert!(
        stderr.contains("`method` is not a `fetch` option"),
        "unexpected diagnostic:\n{stderr}"
    );
    assert!(
        stderr.contains("write `fetch.post(url, ...)`"),
        "the refusal does not say what to write instead:\n{stderr}"
    );
    // The generic mismatch would repeat the complaint without naming the fix.
    assert!(
        !stderr.contains("found `object`"),
        "the generic mismatch was not suppressed:\n{stderr}"
    );
}

/// A program may name its own value `fetch`, and then the methods are its own.
///
/// The recognizer keys on the import rather than on the spelling, which is what
/// keeps a language feature from claiming a name a program already used.
#[test]
fn a_local_named_fetch_keeps_its_own_methods() {
    let root = project_root("http-verbs-shadow");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
interface Mine {
    readonly name: string;
}

const nameOf = (value: &Mine): string => {
    return value.name.copy();
};

function main(): void {
    const fetch: Mine = { name: "local" };
    console.log(nameOf(&fetch));
}
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
        "a local named `fetch` was refused:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(root.join("target/debug").join("http-verbs-shadow"))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "local\n");
}

/// Every way a request can end, each with a code of its own.
///
/// The codes are the point: a name that does not resolve is usually a typo or a
/// stale configuration, a refused connection is often transient and worth
/// retrying, and a handshake that fails is a certificate or protocol problem
/// that retrying will not change. Answering the same code to all three would
/// leave a caller parsing `message` to tell them apart, which is what having a
/// code is for.
///
/// The two named cases are read from an error chain whose wording belongs to
/// the transport, so this test is what turns a rename into a failure rather
/// than a wrong code.
#[test]
fn each_way_a_request_ends_has_a_code_of_its_own() {
    let Some(port) = test_port() else {
        return;
    };
    let Some(plain) = test_port() else {
        return;
    };
    let mut origin = Command::new("python3")
        .arg("-c")
        .arg(
            EDGES
                .replace("EDGE_PORT", &port.to_string())
                .replace("PLAIN_PORT", &plain.to_string()),
        )
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::{BufRead, BufReader};
        let stdout = origin.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line.trim(), "ready", "the origin did not start");
    }

    let root = project_root("http-client-edges");
    build(
        &root,
        "client",
        &CLIENT_EDGES
            .replace("EDGE_PORT", &port.to_string())
            .replace("PLAIN_PORT", &plain.to_string()),
    );
    let output = Command::new(root.join("target/debug").join("http-client-edges"))
        .output()
        .unwrap();
    let _ = origin.kill();
    let _ = origin.wait();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            // A body with no declared length is read to its end.
            "chunked 200 first\n",
            // Within the limit it arrives whole; past it the read stops.
            "large 200 200000\n",
            "too-large ERR_HTTP_FETCH_BODY_TOO_LARGE\n",
            // A server that declares a small length and sends a large body is
            // stopped by the reading limit rather than by the declared one.
            "lying ERR_HTTP_FETCH_BODY_TOO_LARGE\n",
            "timeout ERR_HTTP_FETCH_TIMEOUT\n",
            "dns ERR_HTTP_FETCH_DNS\n",
            "refused ERR_HTTP_FETCH_CONNECT\n",
            "tls ERR_HTTP_FETCH_TLS\n",
            "url ERR_HTTP_FETCH_URL\n",
        )
    );
}

/// Ten requests to one host open one connection.
///
/// The pool is what makes a client worth having, and it was absent: the redirect
/// policy belongs to a client rather than to a request, so a client was built
/// per call and each one brought a pool of its own. Ten requests opened ten
/// connections and, over HTTPS, would have performed ten handshakes. Counted on
/// the server side, because nothing on the client side shows it.
#[test]
fn requests_to_one_host_reuse_the_connection() {
    let Some(port) = test_port() else {
        return;
    };
    let mut counter = Command::new("python3")
        .arg("-c")
        .arg(COUNTER.replace("COUNT_PORT", &port.to_string()))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::{BufRead, BufReader, Write as _};
    let mut reader = BufReader::new(counter.stdout.take().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert_eq!(line.trim(), "ready", "the counter did not start");

    let root = project_root("http-client-reuse");
    build(
        &root,
        "client",
        &r#"
import { fetch, HttpClientResponse, HttpError } from "tyrus:http";

async function once(): Promise<i32> throws HttpError {
    const answer: HttpClientResponse = await fetch.get("http://127.0.0.1:COUNT_PORT/x");
    const status: i32 = answer.status();
    answer.close();
    return status;
}

async function main(): Promise<void> {
    let round: i32 = 0;
    while (round < 10) {
        try {
            const got: i32 = await once();
            if (got !== 200) {
                console.log("unexpected", got);
            }
        } catch (error: HttpError) {
            console.log(error.code);
        }
        round = round + 1;
    }
}
"#
        .replace("COUNT_PORT", &port.to_string()),
    );
    let output = Command::new(root.join("target/debug").join("http-client-reuse"))
        .output()
        .unwrap();
    assert!(
        output.status.success() && output.stdout.is_empty(),
        "the client did not make ten clean requests: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    counter.stdin.take().unwrap().write_all(b"\n").unwrap();
    let mut count = String::new();
    reader.read_line(&mut count).unwrap();
    let _ = counter.kill();
    let _ = counter.wait();
    let _ = fs::remove_dir_all(&root);

    assert_eq!(
        count.trim(),
        "connections=1",
        "ten requests to one host should share one connection"
    );
}

/// A server that answers every path differently, for the edge cases above.
const EDGES: &str = r#"
import socket, sys, threading, time

def serve(conn):
    # The timeout is here so a stuck fixture cannot hang the suite, not to bound
    # a healthy transfer. It applies to each socket operation, including a
    # `sendall` of 200 KB, which blocks whenever the reader is slow: on a
    # saturated machine a value near the transfer time expires mid-body and
    # truncates the response, which reads as a client failure and is not one.
    conn.settimeout(120)
    try:
        request = conn.recv(8192).decode(errors="replace")
        path = request.split(" ")[1] if " " in request else "/"
        if path.startswith("/chunked"):
            # No content-length: the size only appears as it is read.
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n")
            for piece in (b"fi", b"rs", b"t"):
                conn.sendall(b"%x\r\n%s\r\n" % (len(piece), piece))
            conn.sendall(b"0\r\n\r\n")
        elif path.startswith("/big"):
            body = b"x" * 200000
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: %d\r\nConnection: close\r\n\r\n" % len(body) + body)
        elif path.startswith("/lies"):
            # Declares nothing and sends a lot: only the reading limit stops it.
            body = b"y" * 200000
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n")
            conn.sendall(b"%x\r\n%s\r\n" % (len(body), body))
            conn.sendall(b"0\r\n\r\n")
        elif path.startswith("/slow"):
            time.sleep(5)
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
        else:
            conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
    except Exception as error:
        # Never silent: a fixture that gives up has to say so, or its failure
        # is indistinguishable from the client's.
        print(f"fixture error: {error!r}", file=sys.stderr, flush=True)
    finally:
        conn.close()

def listener(port, handler):
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    # The port was free when the test picked it and released it, not reserved,
    # so another process can take it in between; retry rather than fail.
    deadline = time.time() + 10
    while True:
        try:
            srv.bind(("127.0.0.1", port))
            break
        except OSError:
            if time.time() >= deadline:
                raise
            time.sleep(0.05)
    srv.listen(64)
    def loop():
        while True:
            conn, _ = srv.accept()
            threading.Thread(target=handler, args=(conn,), daemon=True).start()
    threading.Thread(target=loop, daemon=True).start()

# The second listener speaks plain HTTP, so an `https` request to it fails in
# the handshake rather than in the connection.
listener(EDGE_PORT, serve)
listener(PLAIN_PORT, serve)
print("ready", flush=True)
threading.Event().wait()
"#;

const CLIENT_EDGES: &str = r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const a: HttpClientResponse = await fetch("http://127.0.0.1:EDGE_PORT/chunked", { timeout: "10s" });
        console.log("chunked", a.status(), a.text());
        a.close();
    } catch (error: HttpError) { console.log("chunked", error.code); }
    try {
        const b: HttpClientResponse = await fetch("http://127.0.0.1:EDGE_PORT/big", { maxBodySize: "1mb" });
        console.log("large", b.status(), b.text().length as i32);
        b.close();
    } catch (error: HttpError) { console.log("large", error.code, error.message); }
    try {
        const c: HttpClientResponse = await fetch("http://127.0.0.1:EDGE_PORT/big", { maxBodySize: "1kb" });
        console.log("too-large", c.status());
        c.close();
    } catch (error: HttpError) { console.log("too-large", error.code); }
    try {
        const d: HttpClientResponse = await fetch("http://127.0.0.1:EDGE_PORT/lies", { maxBodySize: "1kb" });
        console.log("lying", d.status());
        d.close();
    } catch (error: HttpError) { console.log("lying", error.code); }
    try {
        const e: HttpClientResponse = await fetch("http://127.0.0.1:EDGE_PORT/slow", { timeout: "1s" });
        console.log("timeout", e.status());
        e.close();
    } catch (error: HttpError) { console.log("timeout", error.code); }
    try {
        const f: HttpClientResponse = await fetch("http://a-name-that-does-not-exist.invalid/x", { timeout: "5s" });
        console.log("dns", f.status());
        f.close();
    } catch (error: HttpError) { console.log("dns", error.code); }
    try {
        const g: HttpClientResponse = await fetch("http://127.0.0.1:1/x", { timeout: "5s" });
        console.log("refused", g.status());
        g.close();
    } catch (error: HttpError) { console.log("refused", error.code); }
    try {
        const h: HttpClientResponse = await fetch("https://127.0.0.1:PLAIN_PORT/x", { timeout: "5s" });
        console.log("tls", h.status());
        h.close();
    } catch (error: HttpError) { console.log("tls", error.code); }
    try {
        const i: HttpClientResponse = await fetch("not-a-url", { timeout: "5s" });
        console.log("url", i.status());
        i.close();
    } catch (error: HttpError) { console.log("url", error.code); }
}
"#;

/// Counts the connections it accepts, and reports on a line of input.
const COUNTER: &str = r#"
import socket, threading, sys, time

connections = 0
lock = threading.Lock()

def serve(conn):
    global connections
    with lock:
        connections += 1
    # Long enough that a pause between two requests on a kept-alive connection
    # cannot end it on a loaded machine, short enough that the handler still
    # finishes once the client is gone.
    conn.settimeout(30)
    try:
        while True:
            if not conn.recv(4096):
                break
            conn.sendall(
                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n"
                b"Content-Length: 2\r\nConnection: keep-alive\r\n\r\nok"
            )
    except Exception as error:
        print(f"fixture error: {error!r}", file=sys.stderr, flush=True)
    finally:
        conn.close()

srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
# See the note in the edge fixture: the chosen port is free, not reserved.
_deadline = time.time() + 10
while True:
    try:
        srv.bind(("127.0.0.1", COUNT_PORT))
        break
    except OSError:
        if time.time() >= _deadline:
            raise
        time.sleep(0.05)
srv.listen(64)

def loop():
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=serve, args=(conn,), daemon=True).start()

threading.Thread(target=loop, daemon=True).start()
print("ready", flush=True)
sys.stdin.readline()
print(f"connections={connections}", flush=True)
"#;

/// `json<T>()` reads the body through the decoder `T.fromJson(text)` uses.
///
/// One reader, so a response and a string carrying the same document cannot
/// disagree about what it says, nor about which field is wrong when it is: the
/// failure is the same `JsonError`, with the same code and the same path. The
/// roadmap held this back until that shared decoder existed rather than writing
/// a second one for the client, which is what would have made them able to
/// disagree.
#[test]
fn json_reads_the_body_through_the_shared_decoder() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("http-json-server");
    build(
        &server_root,
        "server",
        &SERVER.replace("PORT", &port.to_string()),
    );
    let client_root = project_root("http-json");
    build(
        &client_root,
        "client",
        &r#"
import { fetch, FetchOptions, HttpClientResponse, HttpError, HttpHeader } from "tyrus:http";

interface Echo {
    readonly got: string;
    readonly size: i32;
}

async function main(): Promise<void> {
    try {
        const answer: HttpClientResponse = await fetch.post("http://127.0.0.1:PORT/echo", {
            headers: [{ name: "content-type", value: "text/plain" }],
            body: "read as json",
        });
        const decoded = answer.json<Echo>();
        if (decoded) {
            const echo: Echo = decoded;
            console.log(echo.got, echo.size);
        } else {
            const failure: JsonError = decoded;
            console.log("failed", failure.code);
        }
        answer.close();

        // A body that is not the document the type describes. The refusal is
        // the decoder's own, with the code a `fromJson` would have given.
        const text: HttpClientResponse = await fetch.get("http://127.0.0.1:PORT/hello");
        const wrong = text.json<Echo>();
        if (wrong) {
            console.log("accepted the wrong body");
        } else {
            const failure: JsonError = wrong;
            console.log(failure.code);
        }
        text.close();
    } catch (error: HttpError) {
        console.log("unexpected", error.code);
    }
}
"#
        .replace("PORT", &port.to_string()),
    );

    let server = ServerProcess(Some(
        Command::new(server_root.join("target/debug").join("http-json-server"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));
    wait_until_serving(port);
    let output = Command::new(client_root.join("target/debug").join("http-json"))
        .output()
        .unwrap();
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    let _ = fs::remove_dir_all(&client_root);

    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!("read as json 12\n", "ERR_JSON_INVALID_SYNTAX\n")
    );
}

/// `json` needs the type it reads the body as, and it has to be readable.
#[test]
fn json_without_a_readable_type_says_so() {
    let root = project_root("http-json-untyped");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { fetch, HttpClientResponse, HttpError } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const answer: HttpClientResponse = await fetch.get("http://example.com");
        const decoded = answer.json();
        if (decoded) {
            console.log("read");
        }
        answer.close();
    } catch (error: HttpError) {
        console.log(error.code);
    }
}
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

    assert!(!build.status.success(), "the call should be refused");
    assert!(
        stderr.contains("`json` needs the type to read the body as"),
        "unexpected diagnostic:\n{stderr}"
    );
    assert!(
        stderr.contains("response.json<User>()"),
        "the refusal does not show what to write:\n{stderr}"
    );
}

/// A binding does not need an annotation to say what the call already declares.
///
/// Both shapes here used to fail with "cannot infer a type for binding
/// `<expr>`", an error naming neither the binding nor the call, because code
/// generation kept its own hand-written table of native return types and these
/// were not in it. The types are declared once, in the module's exports; this
/// checks that the answer comes from there. Two shapes rather than one, because
/// a call and an awaited call reach the question by different paths.
#[test]
fn a_native_call_types_its_binding_without_an_annotation() {
    let root = project_root("http-infer-binding");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { createRouter, fetch, HttpError } from "tyrus:http";

const main = async (): Promise<void> => {
    const app = createRouter();
    app.rateLimit({ max: 2, timeWindow: "1 minute" });
    console.log("router");

    try {
        const response = await fetch.get("http://127.0.0.1:9/x");
        // Reached only if something answers on a port nothing listens on;
        // what matters is that `response` typed at all.
        console.log(response.status());
    } catch (error: HttpError) {
        console.log(error.code);
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
        "an unannotated binding was refused:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(root.join("target/debug").join("http-infer-binding"))
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "router\nERR_HTTP_FETCH_CONNECT\n"
    );
}

/// A header read through `??` gives the header's text back.
///
/// `response.header(name)` hands over a `string?` the runtime boxed around a
/// fresh allocation, and `??` clones the payload into its merged result. The
/// box's original was only discarded for the shapes `??` could prove owned,
/// and a method call on a response was not one of them, so every
/// `response.header("x") ?? "y"` leaked the header's text once per read.
///
/// Asserted on the emitted IR: a leak is invisible to any assertion on what
/// the program prints.
#[test]
fn a_coalesced_header_read_gives_back_the_text_it_cloned() {
    let root = project_root("http-client-header-coalesce");
    build(
        &root,
        "client",
        r#"
import { fetch, HttpClientResponse, HttpError } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const response: HttpClientResponse = await fetch("http://127.0.0.1:1/x");
        console.log(response.header("location") ?? "missing");
        response.close();
    } catch (error: HttpError) {
        console.log("unexpected", error.code);
    }
}
"#,
    );
    let ir = fs::read_to_string(
        root.join("target/debug")
            .join("http-client-header-coalesce.ll"),
    )
    .unwrap();
    assert!(
        ir.contains("coalesce.present.discard"),
        "the present branch must release the payload it cloned:\n{ir}"
    );
    let _ = fs::remove_dir_all(&root);
}
