//! `fetch` decodes a compressed response on its own: it offers the codings it
//! can read, decodes the body under the same limits as `tyrus:compress`, and
//! leaves the bytes alone when the program asks it to.

use std::fs;
use std::io::{Read, Write};
use std::process::Command;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn project_root(name: &str) -> std::path::PathBuf {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, Ordering::Relaxed),
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
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "fetch decompression test" }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

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

fn build(root: &std::path::Path, source: &str) {
    fs::write(root.join("src").join("main.tyr"), source).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
}

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

fn run_client(name: &str, source: &str) -> String {
    let root = project_root(name);
    build(&root, source);
    let output = Command::new(root.join("target/debug").join(name))
        .current_dir(&root)
        .output()
        .unwrap();
    let written = fs::read_to_string(root.join("streamed.txt")).unwrap_or_default();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "the client failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    format!("{}{written}", String::from_utf8_lossy(&output.stdout))
}

const SERVER: &str = r#"
import { createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, StreamBody, TextBody } from "tyrus:http";

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/big", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("tyrus ".repeat(2000));
        },
    });
    try {
        app.listen(PORT, { host: "127.0.0.1" });
    } catch (error: HttpError) {
        console.error(`${error.code}: ${error.message}`);
    }
};
"#;

#[test]
fn a_compressed_tyrus_response_arrives_decoded_unless_the_program_opts_out() {
    let Some(port) = test_port() else {
        return;
    };
    let server_root = project_root("fetch-decompression-server");
    build(&server_root, &SERVER.replace("PORT", &port.to_string()));
    let server = ServerProcess(Some(
        Command::new(
            server_root
                .join("target/debug")
                .join("fetch-decompression-server"),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap(),
    ));
    wait_until_serving(port);
    let stdout = run_client(
        "fetch-decompression-client",
        &r#"
import { fetch, FetchOptions, HttpClientResponse, HttpClientStreamResponse, HttpError, HttpHeader } from "tyrus:http";
import { FsError, writeFileStream } from "tyrus:fs";
import { ReadableStream } from "tyrus:stream";

async function main(): Promise<void> {
    const decoded: HttpClientResponse = await fetch("http://127.0.0.1:PORT/big") onfail {
        console.log("unexpected", decoded.code, decoded.message);
        return;
    };
    console.log(decoded.text().length as i32, decoded.header("content-encoding") ?? "decoded");
    decoded.close();

    const raw: HttpClientResponse = await fetch("http://127.0.0.1:PORT/big", {
        headers: [{ name: "accept-encoding", value: "gzip" }],
        decompress: false,
    }) onfail {
        console.log("unexpected", raw.code, raw.message);
        return;
    };
    const bytes: u8[] = raw.bytes();
    console.log(bytes.length < 12000, bytes[0] as i32, bytes[1] as i32, raw.header("content-encoding") ?? "none");
    raw.close();

    const brotli: HttpClientResponse = await fetch("http://127.0.0.1:PORT/big", {
        headers: [{ name: "accept-encoding", value: "br" }],
    }) onfail {
        console.log("unexpected", brotli.code, brotli.message);
        return;
    };
    console.log(brotli.text().length as i32);
    brotli.close();

    const streamed: HttpClientStreamResponse = await fetch.stream("http://127.0.0.1:PORT/big") onfail {
        console.log("unexpected", streamed.code, streamed.message);
        return;
    };
    const body: ReadableStream<u8[]> = await streamed.stream();
    streamed.close();
    const saved = await saveStream(body);
    match (saved) {
        Ok() => {}
        Err(error) => { console.log("unexpected", error.code, error.message); }
    }
}

async function saveStream(body: ReadableStream<u8[]>): Promise<Result<void, FsError>> {
    const destination = writeFileStream("./streamed.txt")?;
    await body.pipeTo(destination);
    return Ok();
}
"#
        .replace("PORT", &port.to_string()),
    );
    drop(server);
    let _ = fs::remove_dir_all(&server_root);
    assert_eq!(
        stdout,
        format!(
            "12000 decoded\ntrue 31 139 gzip\n12000\n{}",
            "tyrus ".repeat(2000)
        )
    );
}

fn gzip(body: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

fn raw_deflate(body: &[u8]) -> Vec<u8> {
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(body).unwrap();
    encoder.finish().unwrap()
}

/// Answers each connection with the response its path names, then closes it.
fn serve_fixtures(listener: std::net::TcpListener) {
    let bomb = gzip(&vec![0_u8; 16 * 1024 * 1024]);
    let raw = raw_deflate(b"raw deflate body");
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        while !request.windows(4).any(|window| window == b"\r\n\r\n") {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => request.extend_from_slice(&buffer[..read]),
            }
        }
        let request = String::from_utf8_lossy(&request);
        let (encoding, body): (&str, &[u8]) = if request.starts_with("GET /bomb") {
            ("gzip", &bomb)
        } else if request.starts_with("GET /liar") {
            ("gzip", b"not compressed at all")
        } else if request.starts_with("GET /raw") {
            ("deflate", &raw)
        } else {
            ("identity", b"offered: ACCEPT")
        };
        let accept = request
            .lines()
            .find_map(|line| {
                line.to_ascii_lowercase()
                    .strip_prefix("accept-encoding: ")
                    .map(str::to_owned)
            })
            .unwrap_or_default();
        let body = if encoding == "identity" {
            String::from_utf8_lossy(body)
                .replace("ACCEPT", &accept)
                .into_bytes()
        } else {
            body.to_vec()
        };
        let head = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-encoding: {encoding}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
    }
}

#[test]
fn a_body_that_lies_or_explodes_is_refused_with_an_http_error() {
    let listener = match std::net::TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: HTTP decompression testing requires permission to bind a local TCP listener ({error})."
            );
            return;
        }
        Err(error) => panic!("could not bind the HTTP decompression test listener: {error}"),
    };
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || serve_fixtures(listener));
    let stdout = run_client(
        "fetch-decompression-fixtures",
        &r#"
import { fetch, FetchOptions, HttpClientResponse, HttpClientStreamResponse, HttpError } from "tyrus:http";
import { ReadableStream, StreamError } from "tyrus:stream";

async function show(path: string): Promise<void> {
    try {
        const response: HttpClientResponse = await fetch(`http://127.0.0.1:PORT${path}`, { maxBodySize: "64mb" });
        console.log(response.text());
        response.close();
    } catch (error: HttpError) {
        console.log(error.code);
    }
}

async function drain(body: ReadableStream<u8[]>): Promise<void> {
    while (true) {
        const piece: u8[]? = await body.read();
        if (piece) {
            continue;
        }
        break;
    }
}

async function streamed(path: string): Promise<void> {
    try {
        const response: HttpClientStreamResponse = await fetch.stream(`http://127.0.0.1:PORT${path}`, { maxBodySize: "64mb" });
        const body: ReadableStream<u8[]> = await response.stream();
        response.close();
        await drain(body);
        console.log("stream ended");
    } catch (error: HttpError | StreamError) {
        console.log("stream", error.code);
    }
}

async function main(): Promise<void> {
    await show("/offered");
    await show("/raw");
    await show("/liar");
    await show("/bomb");
    await streamed("/bomb");
    await streamed("/liar");
}
"#
        .replace("PORT", &port.to_string()),
    );
    assert_eq!(
        stdout,
        "offered: gzip, deflate, br\n\
         raw deflate body\n\
         ERR_HTTP_FETCH_DECODE\n\
         ERR_HTTP_FETCH_RATIO_LIMIT\n\
         stream ERR_HTTP_FETCH_RATIO_LIMIT\n\
         stream ERR_HTTP_FETCH_DECODE\n"
    );
}
