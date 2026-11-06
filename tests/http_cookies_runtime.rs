//! End-to-end coverage for `reply.setCookies([...])` and `reply.clearCookies()`
//! in `tyrus:http`.

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

#[test]
fn a_handler_sets_a_cookie_list_and_clears_the_cookies_the_request_sent() {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => drop(listener),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: HTTP cookie testing requires permission to bind a local TCP listener ({error})."
            );
            return;
        }
        Err(error) => panic!("could not bind the HTTP cookie test listener: {error}"),
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
    let name = format!("http-cookies-{suffix}");
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
    "description": "Reply cookie list integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { CookieOptions, createRouter, fetch, FetchOptions, HttpClientResponse, HttpCookie, HttpError, HttpHeader, HttpReply, HttpRouter, HttpServer, HttpStatus } from "tyrus:http";

interface CookieResponse {
    kept: boolean;
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/session", {
        response: { 200: CookieResponse },
        handler: (request, reply): void => {
            const options: CookieOptions = { path: "/", httpOnly: true };
            const cookies: HttpCookie[] = [
                { name: "theme", value: "dark", options: options },
                { name: "lang", value: "en" },
            ];
            reply.clearCookies().setCookies(cookies);
            // The list is borrowed by the call, so it is still readable here.
            reply.status(HttpStatus.Ok).send({ kept: cookies.length === 2 });
        },
    });
    try {
        const server = app.listen(0) onfail {
            printError(`${server.code}: ${server.message}`);
            return;
        };
        const base: string = `http://127.0.0.1:${server.port()}`;
        const withCookies: HttpClientResponse = await fetch(`${base}/session`, {
            headers: [{ name: "cookie", value: "old=1; stale=2; old=3" }],
        });
        console.log("status", withCookies.status(), withCookies.text());
        console.log("set-cookie", withCookies.header("set-cookie") ?? "none");
        withCookies.close();
        const without: HttpClientResponse = await fetch(`${base}/session`);
        console.log("set-cookie", without.header("set-cookie") ?? "none");
        without.close();
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
    // `header` reads the first `Set-Cookie` value: the clear written first on the
    // request that sent cookies, and the first cookie of the list on the one
    // that sent none. The unit tests cover the full list and its order.
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "status 200 {\"kept\": true}\n\
         set-cookie old=; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT\n\
         set-cookie theme=dark; Path=/; HttpOnly\n"
    );
}
