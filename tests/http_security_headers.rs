//! Security headers, written the way every other response header is.
//!
//! There is no preset that adds them: `reply.header` is the whole mechanism, so
//! what a program sends is what it wrote. This test is what says the ordinary
//! header path carries them verbatim, and it also pins the limitation that
//! comes with the choice: a response the router produced on its own, such as a
//! `404` for an unmatched path, ran no handler and so carries no headers a
//! handler would have set.

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
    "description": "http security headers test"
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

fn send(port: u16, path: &str) -> Option<String> {
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
    Some(raw.to_ascii_lowercase())
}

fn wait_until_serving(port: u16, path: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(response) = send(port, path) {
            return response;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

#[test]
fn a_handler_sends_the_security_headers_it_writes() {
    let root = project_root("http-security-headers");
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
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.header("x-content-type-options", "nosniff");
            reply.header("referrer-policy", "no-referrer");
            reply.header("x-frame-options", "DENY");
            reply.header("cross-origin-resource-policy", "same-origin");
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    app.get("/own", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.header("referrer-policy", "origin");
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

    let mut server = Command::new(root.join("target/debug").join("http-security-headers"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let answered = wait_until_serving(port, "/health");
    // A response the router produced on its own, with no handler involved.
    let missing = send(port, "/nope");
    // A route that wrote a different value for one of them sends that value.
    let overridden = send(port, "/own");

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    for header in [
        "x-content-type-options: nosniff",
        "referrer-policy: no-referrer",
        "x-frame-options: deny",
        "cross-origin-resource-policy: same-origin",
    ] {
        assert!(
            answered.contains(header),
            "`{header}` is missing from the answered response:\n{answered}"
        );
    }

    let missing = missing.expect("the unmatched route was answered");
    assert!(missing.contains("404"), "unexpected response:\n{missing}");
    assert!(
        !missing.contains("x-frame-options"),
        "a response with no handler cannot carry a header a handler sets; if this \
         ever changes, the documented limitation changes with it:\n{missing}"
    );

    // Nothing the handler did not write. A `Content-Security-Policy` belongs to
    // the application, and HSTS is a promise about TLS this server does not
    // terminate: sending either on the program's behalf would be the server
    // deciding something only the program can.
    assert!(
        !answered.contains("content-security-policy"),
        "a header no handler wrote was sent:\n{answered}"
    );
    assert!(
        !answered.contains("strict-transport-security"),
        "a header no handler wrote was sent:\n{answered}"
    );

    let overridden = overridden.expect("the other route was answered");
    assert!(
        overridden.contains("referrer-policy: origin"),
        "a route sends the value it wrote:\n{overridden}"
    );
    assert!(
        !overridden.contains("referrer-policy: no-referrer"),
        "one route's headers leaked into another route's response:\n{overridden}"
    );
}

/// A route that writes none of them sends none of them.
///
/// Without this, a set added behind the program's back would pass every
/// assertion in the test above while making the choice above meaningless: what
/// a program sends has to be what it wrote.
#[test]
fn a_route_that_writes_no_headers_sends_none() {
    let root = project_root("http-no-security-headers");
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
    app.get("/health", {{
        response: {{ 200: Health }},
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

    let mut server = Command::new(root.join("target/debug").join("http-no-security-headers"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let answered = wait_until_serving(port, "/health");

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    for header in [
        "x-content-type-options",
        "referrer-policy",
        "x-frame-options",
        "cross-origin-resource-policy",
    ] {
        assert!(
            !answered.contains(header),
            "`{header}` was sent by a route that never wrote it:\n{answered}"
        );
    }
}
