//! Where a rate limit is written: `app.rateLimit(...)` covers every route, and
//! a route's own `config.rateLimit` replaces that default for itself.
//!
//! The server-wide limit is set on the router, beside `cors` and
//! `trustedProxies`, because it is a policy of the server; `listen` carries the
//! address and the connection.
//!
//! Only a real client shows the difference. A limit that has to be repeated on
//! every route is a limit some route will be missing, so the server-wide form
//! is the one a program reaches for; the per-route form exists for the route
//! that needs a different number, in either direction.

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
    "description": "http rate limit scope test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

/// A port the operating system confirmed is free.
///
/// These test binaries run in parallel, so a fixed or randomly guessed port
/// collides, and the failure is baffling: one test's server answers another
/// test's request, and the assertion reports a body it never asked for. That is
/// exactly how a port picked from the clock landed on a fixed `34_517` here.
/// Binding first and releasing narrows the window to the moment between this
/// call and the server's own bind.
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

fn get(port: u16, path: &str) -> Option<u16> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    raw.split_whitespace().nth(1).and_then(|c| c.parse().ok())
}

fn wait_until_serving(port: u16, path: &str) -> u16 {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(status) = get(port, path) {
            return status;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

#[test]
fn the_server_limit_covers_a_route_and_the_route_can_replace_it() {
    let root = project_root("http-rate-limit-scope");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface Health {{
    readonly ok: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.rateLimit({{ max: 2, timeWindow: "1 minute" }});
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    app.get("/open", {{
        response: {{ 200: Health }},
        config: {{ rateLimit: {{ max: 5, timeWindow: "1 minute" }} }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
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

    let mut server = Command::new(root.join("target/debug").join("http-rate-limit-scope"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // `/health` declares no limit of its own, so it counts against the two the
    // router allows.
    let first = wait_until_serving(port, "/health");
    let second = get(port, "/health");
    let third = get(port, "/health");
    // `/open` declares five, which replaces the server-wide two rather than
    // adding to it: three requests still pass after the default is exhausted.
    let open_first = get(port, "/open");
    let open_second = get(port, "/open");
    let open_third = get(port, "/open");

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    assert_eq!(first, 200);
    assert_eq!(second, Some(200));
    assert_eq!(
        third,
        Some(429),
        "the server rate limit did not reach a route without its own"
    );
    assert_eq!(open_first, Some(200));
    assert_eq!(open_second, Some(200));
    assert_eq!(
        open_third,
        Some(200),
        "a route's own limit must replace the server-wide default, not add to it"
    );
}

/// The limit written where it no longer belongs says where it goes.
///
/// A migration this mechanical deserves a diagnostic that performs it for the
/// reader: the field moved, the replacement has the same shape, and the
/// per-route form did not change at all.
#[test]
fn a_limit_in_the_listen_options_says_where_it_moved() {
    let root = project_root("http-rate-limit-moved");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
import { createRouter, HttpError, HttpRouter, ServerOptions } from "tyrus:http";

const main = (): void => {
    const app: HttpRouter = createRouter();
    try {
        app.listen(0, {
            host: "127.0.0.1",
            rateLimit: { max: 2, timeWindow: "1 minute" },
        });
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

    assert!(!build.status.success(), "the option should be refused");
    assert!(
        stderr.contains("E3294") && stderr.contains("`rateLimit` is a server policy"),
        "unexpected diagnostic:\n{stderr}"
    );
    assert!(
        stderr.contains("app.rateLimit({ max: ..., timeWindow: ... })"),
        "the refusal does not name the replacement:\n{stderr}"
    );
}
