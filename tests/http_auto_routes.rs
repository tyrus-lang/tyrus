//! End-to-end coverage for route plugins discovered under `src/routes/`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct AutoRouteProject {
    root: PathBuf,
    name: String,
}

impl AutoRouteProject {
    fn new() -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("http-auto-routes-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src/routes/nested")).unwrap();
        fs::create_dir_all(root.join("src/controllers")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "HTTP automatic routes integration test"
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { createRouter, fetch, HttpClientResponse, HttpError, HttpRouter, HttpServer, ServerOptions } from "tyrus:http";

async function main(): Promise<void> {
    try {
        const app: HttpRouter = createRouter();
        const server = app.listen(0, { host: "127.0.0.1" }) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const response: HttpClientResponse = await fetch(`http://127.0.0.1:${server.port()}/health`);
        console.log(response.status());
        console.log(response.text());
        response.close();
        await server.close();
    } catch (error: HttpError) {
        printError(error.code);
    }
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("src/controllers/not_a_route.tyr"),
            "export default (value: i32): void => {};",
        )
        .unwrap();
        fs::write(
            root.join("src/routes/nested/ignored.test.tyr"),
            "export default (value: i32): void => {};",
        )
        .unwrap();
        fs::write(
            root.join("src/routes/nested/health.tyr"),
            r#"import { HttpReply, HttpRouter, HttpStatus } from "tyrus:http";

interface HealthResponse {
    status: string;
}

export default (router: HttpRouter): void => {
    router.get("/health", {
        response: { 200: HealthResponse },
        handler: (_request: {}, reply: HttpReply<HealthResponse>): void => {
            reply.status(HttpStatus.Ok).send({ status: "ok" });
        },
    });
};
"#,
        )
        .unwrap();
        Self { root, name }
    }

    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn run(&self) -> Output {
        let binary = self.root.join("target/debug").join(&self.name);
        #[cfg(windows)]
        let binary = binary.with_extension("exe");
        Command::new(binary)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for AutoRouteProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn nested_default_route_plugin_is_registered_before_listen() {
    match std::net::TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => drop(listener),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: HTTP auto route testing requires permission to bind a local TCP listener ({error})."
            );
            return;
        }
        Err(error) => panic!("could not bind the HTTP auto route test listener: {error}"),
    }
    let project = AutoRouteProject::new();
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = project.run();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "200\n{\"status\": \"ok\"}\n"
    );
}

#[test]
fn rejects_default_exports_with_an_invalid_route_plugin_signature() {
    let project = AutoRouteProject::new();
    fs::write(
        project.root.join("src/routes/nested/health.tyr"),
        "export default (port: i32): void => {};",
    )
    .unwrap();

    let build = project.build();
    assert!(
        !build.status.success(),
        "an invalid route plugin unexpectedly built"
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("default export in route module")
            && stderr.contains("(router: HttpRouter) => void"),
        "unexpected route signature diagnostic:\n{stderr}"
    );
}
