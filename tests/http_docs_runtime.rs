//! End-to-end coverage for `app.docs()` in `tyrus:http`.

use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn loopback_bind_is_denied() -> bool {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            drop(listener);
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: HTTP docs testing requires permission to bind a local TCP listener ({error})."
            );
            true
        }
        Err(error) => panic!("could not bind the HTTP docs test listener: {error}"),
    }
}

#[test]
fn a_router_serves_the_openapi_document_of_its_typed_routes() {
    if loopback_bind_is_denied() {
        return;
    }
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("http-docs-{suffix}");
    let root = std::env::temp_dir().join(&name);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "API docs integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { createRouter, fetch, HttpClientResponse, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus } from "tyrus:http";

interface UserParams {
    id: string;
}

interface UserResponse {
    name: string;
    tags: string[];
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.docs({ title: "Users API", version: "2.0.0" });
    app.get("/users/:id", {
        params: UserParams,
        response: { 200: UserResponse },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ name: request.params.id, tags: [] });
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const base: string = `http://127.0.0.1:${server.port()}`;
        const document: HttpClientResponse = await fetch(`${base}/docs/openapi.json`);
        console.log("document", document.status());
        const text: string = document.text();
        console.log("path", text.includes("\"/users/{id}\""));
        console.log("title", text.includes("\"Users API\""));
        console.log("schema", text.includes("\"UserResponse\""));
        document.close();
        const page: HttpClientResponse = await fetch(`${base}/docs`);
        console.log("page", page.status(), page.text().includes("swagger-ui-bundle.js"));
        page.close();
        await server.close();
    } catch (error: HttpError) {
        console.error(error.code);
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
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "document 200\npath true\ntitle true\nschema true\npage 200 true\n"
    );
}

#[test]
fn the_document_carries_auth_examples_websocket_routes_and_the_page_its_css() {
    if loopback_bind_is_denied() {
        return;
    }
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("http-docs-auth-{suffix}");
    let root = std::env::temp_dir().join(&name);
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "API docs auth integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { createRouter, fetch, HttpClientResponse, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, WebSocket } from "tyrus:http";

interface CreateBook {
    @minLength(3)
    title: string;
    @min(1)
    pages: i32;
}

interface BookResponse {
    title: string;
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.docs({
        auth: { bearer: true, bearerFormat: "JWT", apiKey: { name: "x-api-key", in: "header" } },
        css: ".topbar { display: none; }",
    });
    app.post("/books", {
        body: CreateBook,
        response: { 201: BookResponse },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Created).send({ title: request.body.title });
        },
    });
    app.ws("/rooms/:room", {
        handler: async (socket: WebSocket): Promise<void> => {
            await socket.close();
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const base: string = `http://127.0.0.1:${server.port()}`;
        const document: HttpClientResponse = await fetch(`${base}/docs/openapi.json`);
        const text: string = document.text();
        console.log("bearer", text.includes("\"bearerFormat\":\"JWT\""));
        console.log("apiKey", text.includes("\"apiKeyAuth\""));
        console.log("security", text.includes("\"security\":["));
        console.log(
            "example",
            text.includes("\"example\":{")
                && text.includes("\"pages\":1")
                && text.includes("\"title\":\"string\""),
        );
        console.log("websocket", text.includes("\"/rooms/{room}\"") && text.includes("\"x-tyrus-websocket\":true"));
        document.close();
        const page: HttpClientResponse = await fetch(`${base}/docs`);
        const html: string = page.text();
        console.log("css", html.includes(".topbar { display: none; }"));
        console.log("persist", html.includes("persistAuthorization: true"));
        page.close();
        await server.close();
    } catch (error: HttpError) {
        console.error(error.code);
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
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "bearer true\napiKey true\nsecurity true\nexample true\nwebsocket true\ncss true\npersist true\n"
    );
}
