//! A response that breaks its route's schema, and why it is now hard to write.
//!
//! This started as an end-to-end reproduction: a decorated field assigned after
//! construction produced an object the schema refused, and the server answered
//! `500`. The assignment gap that made it possible is closed, so the same
//! program is now refused while it is still source, which is where a rule about
//! a field belongs.
//!
//! What is left here is that refusal. The `500` itself still has to behave, and
//! that is covered by the runtime test in `crates/tyrus-runtime`, which drives
//! the response path directly rather than through a program the compiler no
//! longer accepts.

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
    "description": "http response schema test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

/// A port the operating system confirmed is free. See the note in
/// `http_server_lifecycle.rs`.
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

#[test]
fn a_field_assignment_that_breaks_its_decoration_is_refused() {
    let name = "http-response-schema";
    let root = project_root(name);
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface Health {{
    @minLength(5)
    name: string;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            // Built valid, then assigned a value the rule refuses. This used to
            // compile, hand the router an object its own schema rejected, and
            // surface as a `500` at run time.
            let value: Health = {{ name: "valid-name" }};
            value.name = "ab";
            reply.status(HttpStatus.Ok).send(value);
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
    let diagnostics = String::from_utf8_lossy(&build.stderr).to_string();
    let _ = fs::remove_dir_all(&root);

    assert!(
        !build.status.success(),
        "a value the field's own rule refuses was accepted:\n{diagnostics}"
    );
    assert!(
        diagnostics.contains("E3165"),
        "expected the decoration to reject the assigned value:\n{diagnostics}"
    );
    assert!(
        diagnostics.contains("@minLength(5)") && diagnostics.contains("name"),
        "the diagnostic did not name the rule and the field:\n{diagnostics}"
    );
    // The offending value, not the declaration that was valid when written.
    assert!(
        diagnostics.contains("value.name = \"ab\";"),
        "the diagnostic did not point at the assignment:\n{diagnostics}"
    );
}
