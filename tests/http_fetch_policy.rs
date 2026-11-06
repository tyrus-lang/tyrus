//! Where `fetch` is allowed to go, and how that is written down.
//!
//! The attack these tests describe is the one a route invites when it hands
//! `fetch` a URL the client chose: the request then leaves from inside the
//! network, so it reaches whatever the network was protecting.
//! `169.254.169.254` is the cloud metadata service, which answers credentials
//! to anyone who can reach it.
//!
//! Two gates have to hold, and neither is enough alone. A URL naming an address
//! outright never reaches a resolver, so it is checked before dispatch. A URL
//! naming a host is checked by what that host resolves to, because a name check
//! is not a check: nothing stops a public name from resolving to a private
//! address, and a policy that trusted the spelling would approve the metadata
//! service the moment someone pointed a domain at it.
//!
//! Every test that asserts a refusal is paired with one that asserts the same
//! request succeeds without the policy. A refusal that would have happened
//! anyway proves nothing, and "connection refused" and "the policy said no"
//! look identical from a program that only checks that it failed.

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
    "description": "fetch policy test"
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

/// Compiles the program and returns what it printed.
fn run(name: &str, source: &str) -> String {
    let root = project_root(name);
    fs::write(root.join("src").join("main.tyr"), source).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "building `{name}` failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let output = Command::new(root.join("target/debug").join(name))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "`{name}` exited with {}:\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    stdout
}

/// The probe every program below uses: it reports what happened by code, so a
/// refusal cannot be confused with a connection that failed on its own.
const PROBE: &str = r#"
const probe = async (label: string, url: string): Promise<void> => {
    try {
        const response = await fetch.get(url);
        console.log(`${label}: reached ${response.status()}`);
    } catch (error: HttpError) {
        console.log(`${label}: ${error.code}`);
    }
};
"#;

#[test]
fn a_private_destination_is_refused_in_every_notation() {
    let output = run(
        "fetch-policy-refuses",
        &format!(
            r#"
import {{ createRouter, fetch, FetchPolicyOptions, HttpError, HttpRouter }} from "tyrus:http";
{PROBE}
const main = async (): Promise<void> => {{
    const app: HttpRouter = createRouter();
    app.fetchPolicy({{ allow: [], denyPrivate: true }});
    await probe("metadata", "http://169.254.169.254/latest/meta-data/");
    await probe("mapped", "http://[::ffff:169.254.169.254]/latest/meta-data/");
    await probe("loopback", "http://127.0.0.1:9/x");
    await probe("private", "http://10.0.0.5:9/x");
    await probe("by name", "http://localhost:9/x");
}};
"#
        ),
    );

    // The last one is the whole reason the filter lives in the resolver:
    // `localhost` is a name, so nothing about the URL says it is loopback.
    assert_eq!(
        output,
        "metadata: ERR_HTTP_FETCH_BLOCKED\n\
         mapped: ERR_HTTP_FETCH_BLOCKED\n\
         loopback: ERR_HTTP_FETCH_BLOCKED\n\
         private: ERR_HTTP_FETCH_BLOCKED\n\
         by name: ERR_HTTP_FETCH_BLOCKED\n",
    );
}

/// Without the policy the same requests are not refused.
///
/// This is what separates a working filter from a `fetch` that stopped working:
/// every code here is a transport answer, and none of them is the policy's.
#[test]
fn a_program_that_declares_no_policy_is_not_restricted() {
    let output = run(
        "fetch-policy-absent",
        &format!(
            r#"
import {{ fetch, HttpError }} from "tyrus:http";
{PROBE}
const main = async (): Promise<void> => {{
    await probe("loopback", "http://127.0.0.1:9/x");
    await probe("by name", "http://localhost:9/x");
}};
"#
        ),
    );

    assert!(
        !output.contains("ERR_HTTP_FETCH_BLOCKED"),
        "nothing may be refused before a program asks for it:\n{output}"
    );
    assert!(
        output.contains("loopback: ERR_HTTP_FETCH_CONNECT"),
        "the request should have failed as an ordinary connection:\n{output}"
    );
}

/// An exemption has to actually reach, not merely fail differently.
///
/// So the exempt host is a Tyrus server that answers, and the assertion is on
/// the body it wrote. A test that only checked the code would pass just as well
/// if the exemption produced a connect error instead of a refusal.
#[test]
fn an_exempt_host_is_reached_while_the_rest_stays_refused() {
    let Some(port) = test_port() else {
        return;
    };
    let root = project_root("fetch-policy-server");
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions, TextBody }} from "tyrus:http";

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/hello", {{
        response: {{ 200: TextBody }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send("reached the exempt host");
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
        "building the server failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut server = Command::new(root.join("target/debug").join("fetch-policy-server"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let output = run(
        "fetch-policy-exempt",
        &format!(
            r#"
import {{ createRouter, fetch, FetchPolicyOptions, HttpError, HttpRouter }} from "tyrus:http";

const main = async (): Promise<void> => {{
    const app: HttpRouter = createRouter();
    app.fetchPolicy({{ allow: ["localhost"], denyPrivate: true }});
    try {{
        const response = await fetch.get("http://localhost:{port}/hello");
        console.log(`exempt: ${{response.text()}}`);
    }} catch (error: HttpError) {{
        console.log(`exempt: ${{error.code}}`);
    }}
    try {{
        // The same server, named by the address the exemption does not cover.
        const other = await fetch.get("http://127.0.0.1:{port}/hello");
        console.log(`literal: reached ${{other.status()}}`);
    }} catch (error: HttpError) {{
        console.log(`literal: ${{error.code}}`);
    }}
}};
"#
        ),
    );

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    assert_eq!(
        output, "exempt: reached the exempt host\nliteral: ERR_HTTP_FETCH_BLOCKED\n",
        "the exemption must reach the host it names and cover nothing else"
    );
}

/// A redirect is a destination the program never wrote.
///
/// That makes it the interesting way in: the caller asks for a host it is
/// allowed to reach, and the answer points somewhere it is not. The first URL
/// passing the gate says nothing about the second, so every hop is checked.
#[test]
fn a_redirect_into_a_refused_destination_is_stopped() {
    let Some(port) = test_port() else {
        return;
    };
    let Some(target) = test_port() else {
        return;
    };
    let listener = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
    let redirector = std::thread::spawn(move || {
        // Exactly one connection: the client asks this host once, and the
        // address it is redirected to is refused before anything connects
        // there. Waiting for a second would wait forever.
        if let Some(Ok(mut stream)) = listener.incoming().next() {
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:{target}/x\r\n\
                     Content-Length: 0\r\nConnection: close\r\n\r\n"
                )
                .as_bytes(),
            );
        }
    });

    let output = run(
        "fetch-policy-redirect",
        &format!(
            r#"
import {{ createRouter, fetch, FetchPolicyOptions, HttpError, HttpRouter }} from "tyrus:http";
{PROBE}
const main = async (): Promise<void> => {{
    const app: HttpRouter = createRouter();
    // `localhost` may be reached; the address the redirect names may not.
    app.fetchPolicy({{ allow: ["localhost"], denyPrivate: true }});
    await probe("redirected", "http://localhost:{port}/away");
}};
"#
        ),
    );
    let _ = redirector.join();

    assert_eq!(
        output, "redirected: ERR_HTTP_FETCH_BLOCKED\n",
        "the hop the program never wrote must be checked like the one it did"
    );
}

/// The refusal is its own code.
///
/// A blocked destination reaches the transport as a resolution that produced
/// nothing, which reads as a DNS failure. Reporting it that way would tell a
/// caller their host name is wrong when the truth is that their own policy
/// refused it, and that is a wrong answer a program cannot recover from.
#[test]
fn a_refusal_is_not_reported_as_a_name_that_does_not_resolve() {
    let output = run(
        "fetch-policy-code",
        &format!(
            r#"
import {{ createRouter, fetch, FetchPolicyOptions, HttpError, HttpRouter }} from "tyrus:http";
{PROBE}
const main = async (): Promise<void> => {{
    const app: HttpRouter = createRouter();
    app.fetchPolicy({{ allow: [], denyPrivate: true }});
    await probe("refused", "http://localhost:9/x");
    await probe("unresolvable", "http://tyrus.invalid./x");
}};
"#
        ),
    );

    assert_eq!(
        output, "refused: ERR_HTTP_FETCH_BLOCKED\nunresolvable: ERR_HTTP_FETCH_DNS\n",
        "the two must stay distinguishable:\n{output}"
    );
}

/// The alternate spellings of an IPv4 address are the classic way past a filter
/// like this one, and they are refused here.
///
/// `http://2130706433/` is `127.0.0.1` written as one decimal number, and the
/// hexadecimal, octal and short forms are the same address again. What refuses
/// them is that the URL is parsed into a canonical address before anything
/// looks at it, which means this is a guarantee inherited rather than written,
/// and an inherited guarantee is the kind a dependency bump removes with no
/// other symptom: the request still works, the body still arrives, and the only
/// difference is that the policy stopped applying. So it is pinned.
#[test]
fn an_address_written_another_way_is_still_that_address() {
    let output = run(
        "fetch-policy-notation",
        &format!(
            r#"
import {{ createRouter, fetch, FetchPolicyOptions, HttpError, HttpRouter }} from "tyrus:http";
{PROBE}
const main = async (): Promise<void> => {{
    const app: HttpRouter = createRouter();
    app.fetchPolicy({{ allow: [], denyPrivate: true }});
    await probe("decimal", "http://2130706433:9/x");
    await probe("hex", "http://0x7f000001:9/x");
    await probe("octal", "http://0177.0.0.1:9/x");
    await probe("short", "http://127.1:9/x");
    await probe("metadata decimal", "http://2852039166/x");
}};
"#
        ),
    );

    assert_eq!(
        output,
        "decimal: ERR_HTTP_FETCH_BLOCKED\n\
         hex: ERR_HTTP_FETCH_BLOCKED\n\
         octal: ERR_HTTP_FETCH_BLOCKED\n\
         short: ERR_HTTP_FETCH_BLOCKED\n\
         metadata decimal: ERR_HTTP_FETCH_BLOCKED\n",
    );
}
