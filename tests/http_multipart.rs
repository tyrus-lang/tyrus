//! Reading a `multipart/form-data` request body.
//!
//! The parse happens in the runtime's dispatch, beside the body limit and the
//! content type check, because everything it can refuse is a policy of the
//! route rather than of the program the route calls. These tests drive a real
//! server so the refusals are the ones a client actually receives.

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
    "description": "http multipart test"
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

/// Sends a request whose body is written byte for byte, and returns the status
/// line plus the body.
///
/// The body is assembled here rather than by a client library on purpose: a
/// test for a parser has to be able to send a body the parser should refuse.
fn post(port: u16, path: &str, content_type: &str, body: &[u8]) -> Option<(u16, Vec<u8>)> {
    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .ok()?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n",
        if content_type.is_empty() {
            String::new()
        } else {
            format!("Content-Type: {content_type}\r\n")
        },
        body.len()
    );
    socket.write_all(head.as_bytes()).ok()?;
    socket.write_all(body).ok()?;
    let mut raw = Vec::new();
    let _ = socket.read_to_end(&mut raw);
    if raw.is_empty() {
        return None;
    }
    let separator = raw.windows(4).position(|window| window == b"\r\n\r\n")?;
    let head = String::from_utf8_lossy(&raw[..separator]).to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<u16>().ok())?;
    Some((status, raw[separator + 4..].to_vec()))
}

const BOUNDARY: &str = "----tyrusboundary";

/// One part, with an optional file name and content type.
fn part(name: &str, file_name: Option<&str>, content_type: Option<&str>, body: &[u8]) -> Vec<u8> {
    let mut out = format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"");
    if let Some(file_name) = file_name {
        out.push_str(&format!("; filename=\"{file_name}\""));
    }
    out.push_str("\r\n");
    if let Some(content_type) = content_type {
        out.push_str(&format!("Content-Type: {content_type}\r\n"));
    }
    out.push_str("\r\n");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(body);
    bytes.extend_from_slice(b"\r\n");
    bytes
}

fn body(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(part);
    }
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

fn multipart_content_type() -> String {
    format!("multipart/form-data; boundary={BOUNDARY}")
}

const SOURCE: &str = r#"
import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, MultipartBody, MultipartPart, ServerOptions } from "tyrus:http";

interface Received {
    count: i32;
    firstName: string;
    firstFile: string;
    firstType: string;
    firstSize: i32;
    firstByte: i32;
    secondName: string;
    secondFile: string;
}

const fileNameOf = (part: &MultipartPart): string => {
    if (part.fileName) {
        return part.fileName;
    }
    return "<none>";
};

const contentTypeOf = (part: &MultipartPart): string => {
    if (part.contentType) {
        return part.contentType;
    }
    return "<none>";
};

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.post("/upload", {
        body: MultipartBody,
        response: { 200: Received },
        handler: (request, reply): void => {
            const parts: MultipartPart[] = request.body.copy();
            reply.status(HttpStatus.Ok).send({
                count: parts.length as i32,
                firstName: parts[0].name.copy(),
                firstFile: fileNameOf(&parts[0]),
                firstType: contentTypeOf(&parts[0]),
                firstSize: parts[0].bytes.length as i32,
                firstByte: parts[0].bytes[0] as i32,
                secondName: parts[1].name.copy(),
                secondFile: fileNameOf(&parts[1]),
            });
        },
    });
    try {
        app.listen(PORT, { host: "127.0.0.1" });
    } catch (error: HttpError) {
        console.error(`${error.code}: ${error.message}`);
    }
};
"#;

/// A real upload, and every refusal the route can answer with.
///
/// One server for all of them: a refusal that only holds because the route was
/// built alone would not be worth much, and the accepted upload right beside
/// them shows the route is otherwise working.
#[test]
fn a_multipart_body_is_read_into_parts_and_bad_ones_are_refused() {
    let root = project_root("http-multipart");
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
        Command::new(root.join("target/debug").join("http-multipart"))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    ));

    // 0x89 and 0x00 are what a text representation would have destroyed.
    let picture: [u8; 6] = [0x89, b'P', b'N', b'G', 0x00, 0xFF];
    let upload = body(&[
        part("avatar", Some("pic.png"), Some("image/png"), &picture),
        part("nickname", None, None, b"grace"),
    ]);

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut accepted = None;
    while Instant::now() < deadline {
        if let Some(response) = post(port, "/upload", &multipart_content_type(), &upload) {
            accepted = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // A file name that tries to climb out of whatever directory the program
    // picks. Only the last segment reaches the program, so it cannot.
    let traversal = body(&[
        part(
            "avatar",
            Some("../../etc/passwd"),
            Some("image/png"),
            &picture,
        ),
        part("nickname", None, None, b"grace"),
    ]);
    let traversal = post(port, "/upload", &multipart_content_type(), &traversal);

    // A body in another format entirely, and one that does not say.
    let wrong_type = post(port, "/upload", "application/json", br#"{"a":1}"#);
    let no_type = post(port, "/upload", "", b"whatever");
    // A boundary that nothing in the body matches.
    let malformed = post(
        port,
        "/upload",
        &multipart_content_type(),
        b"not a multipart body at all",
    );
    // More parts than a route accepts.
    let many: Vec<Vec<u8>> = (0..200)
        .map(|index| part(&format!("f{index}"), None, None, b"v"))
        .collect();
    let too_many = post(port, "/upload", &multipart_content_type(), &body(&many));

    drop(server);
    let _ = fs::remove_dir_all(&root);

    let (status, answer) = accepted.expect("the server never answered");
    assert_eq!(status, 200, "a valid upload was refused");
    let actual: serde_json::Value =
        serde_json::from_slice(&answer).expect("the valid multipart response should contain JSON");
    assert_eq!(
        actual,
        serde_json::json!({
            "count": 2,
            "firstName": "avatar",
            "firstFile": "pic.png",
            "firstType": "image/png",
            "firstSize": 6,
            "firstByte": 137,
            "secondName": "nickname",
            "secondFile": "<none>"
        }),
        "the parts did not arrive as they were sent"
    );

    let (status, answer) = traversal.expect("no answer for the traversal attempt");
    assert_eq!(status, 200);
    assert!(
        String::from_utf8_lossy(&answer).contains("\"firstFile\": \"passwd\""),
        "a file name reached the program still naming a directory:\n{}",
        String::from_utf8_lossy(&answer)
    );

    assert_eq!(
        wrong_type.expect("no answer for the wrong content type").0,
        415,
        "a route reading parts accepted JSON"
    );
    assert_eq!(
        no_type.expect("no answer for the missing content type").0,
        415,
        "a body with no content type has no boundary to be found in"
    );
    assert_eq!(
        malformed.expect("no answer for the malformed body").0,
        400,
        "a body that does not parse is the sender's data, not its format"
    );
    assert_eq!(
        too_many.expect("no answer for the oversized body").0,
        413,
        "a body under the size limit can still be too many parts"
    );
}

/// `MultipartBody` reads a request and cannot answer one.
///
/// A server that replies in multipart is rare enough that guessing at the
/// contract would be worse than not offering one, so the refusal is explicit
/// rather than a shape that compiles into something unhelpful.
#[test]
fn multipart_is_refused_as_a_response() {
    let root = project_root("http-multipart-response");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, MultipartBody, ServerOptions } from "tyrus:http";

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/a", {
        response: { 200: MultipartBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("x");
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

    assert!(!build.status.success(), "the route should be refused");
    assert!(
        stderr.contains("`MultipartBody` reads a request body, so a `response` cannot be one"),
        "unexpected diagnostic:\n{stderr}"
    );
}
