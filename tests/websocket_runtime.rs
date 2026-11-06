//! End-to-end coverage for WebSocket routes and clients in `tyrus:http`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn loopback_bind_is_denied() -> bool {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            drop(listener);
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: WebSocket testing requires permission to bind a local TCP listener ({error})."
            );
            true
        }
        Err(error) => panic!("could not bind the WebSocket test listener: {error}"),
    }
}

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn build_and_run(name_prefix: &str, source: &str) -> String {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("{name_prefix}-{suffix}");
    let root: PathBuf = std::env::temp_dir().join(&name);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "WebSocket integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(root.join("src/main.tyr"), source).unwrap();
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
    let binary = root.join("target/debug").join(&name);
    #[cfg(windows)]
    let binary = binary.with_extension("exe");
    let output = Command::new(binary).current_dir(&root).output().unwrap();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&root);
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_websocket_route_and_client_exchange_text_and_binary_messages() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-echo",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket, WebSocketMessage } from "tyrus:http";

async function echoMessages(socket: WebSocket): Promise<void> throws HttpError {
    let open: boolean = true;
    while (open) {
        const message: WebSocketMessage? = await socket.receive() onfail { throw message; };
        if (message) {
            if (message.binary) {
                const sent = await socket.sendBytes(message.bytes) onfail { throw sent; };
            } else {
                const sent = await socket.sendText(`echo ${message.text}`) onfail { throw sent; };
            }
        } else {
            open = false;
        }
    }
}

async function echo(socket: WebSocket): Promise<void> {
    try {
        await echoMessages(socket);
    } catch (error: HttpError) {
        console.error("server", error.code);
    }
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/echo", { handler: echo, subprotocols: ["chat"] });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const port: i32 = server.port();
        const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${port}/echo`, ["chat"]) onfail { throw socket; };
        console.log("protocol", socket.protocol());
        const sentText = await socket.sendText("hello") onfail { throw sentText; };
        const reply: WebSocketMessage? = await socket.receive() onfail { throw reply; };
        if (reply) {
            console.log("text", reply.text, reply.binary);
        }
        const sentBytes = await socket.sendBytes([1, 2, 3]) onfail { throw sentBytes; };
        const bytes: WebSocketMessage? = await socket.receive() onfail { throw bytes; };
        if (bytes) {
            console.log("bytes", bytes.bytes.length, bytes.binary);
        }
        const closed = await socket.close() onfail { throw closed; };
        await server.close();
        console.log("done");
    } catch (error: HttpError) {
        console.error("client", error.code);
    }
}
"#,
    );
    assert_eq!(
        stdout,
        "protocol chat\ntext echo hello false\nbytes 3 true\ndone\n"
    );
}

#[test]
fn a_websocket_binary_duplex_preserves_whole_messages_end_to_end() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-duplex",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket } from "tyrus:http";
import { DuplexStream, StreamError } from "tyrus:stream";

async function echo(socket: WebSocket): Promise<void> throws HttpError | StreamError {
    const duplex: DuplexStream<u8[], u8[]> = await socket.stream() onfail { throw duplex; };
    const message: u8[]? = await duplex.readable.read();
    if (message) {
        await duplex.writable.write(message);
    }
    await duplex.writable.close();
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/echo", { handler: echo });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${server.port()}/echo`) onfail { throw socket; };
        const duplex: DuplexStream<u8[], u8[]> = await socket.stream() onfail { throw duplex; };
        await duplex.writable.write([4, 5, 6]);
        const reply: u8[]? = await duplex.readable.read();
        if (reply) {
            console.log("reply", reply.length, reply[0], reply[2]);
        }
        await duplex.writable.close();
        const eof: u8[]? = await duplex.readable.read();
        console.log("eof", eof ?? []);
        await server.close();
    } catch (error: HttpError | StreamError) {
        console.error("client", error.code);
    }
}
"#,
    );
    assert_eq!(stdout, "reply 3 4 6\neof []\n");
}

#[test]
fn an_inline_websocket_handler_that_closes_ends_the_client_receive() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-inline-close",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket, WebSocketMessage } from "tyrus:http";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/bye", {
        handler: async (socket: WebSocket): Promise<void> throws HttpError => {
            const closed = await socket.close() onfail { throw closed; };
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${server.port()}/bye`) onfail { throw socket; };
        const message: WebSocketMessage? = await socket.receive() onfail { throw message; };
        console.log("closed", message ? false : true);
        console.log("close-code", socket.closeCode() ?? -1);
        await server.close();
    } catch (error: HttpError) {
        console.error("client", error.code);
    }
}
"#,
    );
    assert_eq!(stdout, "closed true\nclose-code 1000\n");
}

#[test]
fn a_generated_server_shutdown_closes_a_pending_websocket_handler() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-shutdown",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket, WebSocketMessage } from "tyrus:http";

async function waitForClose(socket: WebSocket): Promise<void> throws HttpError {
    const message: WebSocketMessage? = await socket.receive() onfail { throw message; };
    console.log("handler", message ? "message" : "closed");
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/pending", { handler: waitForClose });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${server.port()}/pending`) onfail { throw socket; };
        await server.close();
        const message: WebSocketMessage? = await socket.receive() onfail { throw message; };
        console.log("client", message ? "message" : "closed", socket.closeCode() ?? -1);
    } catch (error: HttpError) {
        console.error("unexpected", error.code);
    }
}
"#,
    );
    assert_eq!(stdout, "handler closed\nclient closed 1001\n");
}

#[test]
fn a_websocket_connection_refused_rejects_with_a_stable_code() {
    let stdout = build_and_run(
        "websocket-refused",
        r#"import { connectWebSocket, HttpError, WebSocket } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const socket: WebSocket = await connectWebSocket("http://127.0.0.1:1/") onfail { throw socket; };
        const closed = await socket.close() onfail { throw closed; };
    } catch (error: HttpError) {
        console.log(error.code);
    }
}
"#,
    );
    assert_eq!(stdout, "ERR_HTTP_WS_URL\n");
}

#[test]
fn a_websocket_close_takes_an_application_code_and_refuses_a_reserved_one() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-close-code",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRouter, HttpServer, WebSocket, WebSocketMessage } from "tyrus:http";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/bye", {
        handler: async (socket: WebSocket): Promise<void> throws HttpError => {
            try {
                const closed = await socket.close(1001, "reserved") onfail { throw closed; };
            } catch (error: HttpError) {
                console.log("server", error.code);
            }
            try {
                const closed = await socket.close(4000, "done") onfail { throw closed; };
            } catch (error: HttpError) {
                console.log("unexpected", error.code);
            }
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${server.port()}/bye`) onfail { throw socket; };
        const message: WebSocketMessage? = await socket.receive() onfail { throw message; };
        console.log("closed", message ? false : true);
        console.log("close-code", socket.closeCode() ?? -1);
        const closed = await socket.close() onfail { throw closed; };
        await server.close();
    } catch (error: HttpError) {
        console.error("client", error.code);
    }
}
"#,
    );
    assert_eq!(
        stdout,
        "server ERR_HTTP_WS_CLOSE_CODE\nclosed true\nclose-code 4000\n"
    );
}

#[test]
fn a_websocket_pre_handler_refuses_an_unauthorized_upgrade() {
    if loopback_bind_is_denied() {
        return;
    }
    let stdout = build_and_run(
        "websocket-pre-handler",
        r#"import { connectWebSocket, createRouter, HttpError, HttpRequest, HttpResponse, HttpRouter, HttpServer, HttpStatus, WebSocket } from "tyrus:http";

function requireAuthorization(request: HttpRequest): HttpResponse | true {
    if (request.headers.includes("authorization")) {
        return true;
    }
    return { statusCode: HttpStatus.Unauthorized, body: "Authorization header is required", cookies: "" };
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.ws("/private", {
        preHandler: [requireAuthorization],
        handler: async (socket: WebSocket): Promise<void> throws HttpError => {
            const closed = await socket.close() onfail { throw closed; };
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        try {
            const socket: WebSocket = await connectWebSocket(`ws://127.0.0.1:${server.port()}/private`) onfail { throw socket; };
            console.log("connected");
            const closed = await socket.close() onfail { throw closed; };
        } catch (error: HttpError) {
            console.log(error.code);
        }
        await server.close();
    } catch (error: HttpError) {
        console.error("server", error.code);
    }
}
"#,
    );
    assert_eq!(stdout, "ERR_HTTP_WS_HANDSHAKE\n");
}
