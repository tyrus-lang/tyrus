//! Every request channel of one route, accepted and rejected.
//!
//! The unit tests reach the runtime directly and the semantic tests stop at the
//! type checker, so neither covers what a real client sees when a route
//! declares `params`, `query`, `headers`, `cookies` and `body` together: the
//! compiler generates one wrapper that reads all five, and a failure in any of
//! them has to name that channel and reject the request without touching the
//! handler.

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
    "description": "http request channel test"
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

struct Response {
    status: u16,
    body: String,
}

fn send(port: u16, request: &str) -> Option<Response> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        if stream.write_all(request.as_bytes()).is_err() {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let mut raw = String::new();
        if stream.read_to_string(&mut raw).is_err() {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }
        let status = raw
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse().ok())?;
        let body = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.trim().to_string())
            .unwrap_or_default();
        return Some(Response { status, body });
    }
    None
}

/// One `POST /users/:id` carrying a value in every channel, plus one variant
/// per channel with that channel's value made invalid.
fn channel_request(
    id: &str,
    query: &str,
    authorization: &str,
    session: &str,
    body: &str,
) -> String {
    format!(
        "POST /users/{id}?{query} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Content-Type: application/json\r\n\
         Authorization: {authorization}\r\n\
         Cookie: session={session}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

#[test]
fn every_request_channel_is_read_and_rejected_on_its_own() {
    let root = project_root("http-request-channels");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface UserParams {{
    @minLength(3)
    id: string;
}}

interface UserQuery {{
    limit: i32;
    verbose?: boolean;
}}

interface AuthHeaders {{
    @minLength(6)
    authorization: string;
}}

interface UserCookies {{
    @minLength(4)
    session: string;
}}

interface CreateBody {{
    @minLength(2)
    name: string;

    @email()
    email: string;
}}

interface CreatedResponse {{
    id: string;
    limit: i32;
    verbose: boolean;
    authorization: string;
    session: string;
    name: string;
    email: string;
}}

type CreateRequest = {{
    params: UserParams;
    query: UserQuery;
    headers: AuthHeaders;
    cookies: UserCookies;
    body: CreateBody;
}};

const createUser = (request: CreateRequest, reply: HttpReply<CreatedResponse>): void => {{
    reply.status(HttpStatus.Created).send({{
        id: request.params.id,
        limit: request.query.limit,
        verbose: request.query.verbose ?? false,
        authorization: request.headers.authorization,
        session: request.cookies.session,
        name: request.body.name,
        email: request.body.email,
    }});
}};

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.post("/users/:id", {{
        params: UserParams,
        query: UserQuery,
        headers: AuthHeaders,
        cookies: UserCookies,
        body: CreateBody,
        response: {{ 201: CreatedResponse }},
        handler: createUser,
    }});
    try {{
        app.listen({port}, {{ host: "127.0.0.1" }});
    }} catch (error: HttpError) {{
        console.error(error.code);
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

    let mut server = Command::new(root.join("target/debug").join("http-request-channels"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let valid_body = r#"{"name":"Ada","email":"ada@example.com"}"#;
    let accepted = send(
        port,
        &channel_request(
            "user-1",
            "limit=25&verbose=true",
            "Bearer token",
            "abcd",
            valid_body,
        ),
    );

    // One rejection per channel, each with only that channel's value invalid.
    let bad_params = send(
        port,
        &channel_request("ab", "limit=25", "Bearer token", "abcd", valid_body),
    );
    let bad_query = send(
        port,
        &channel_request("user-1", "limit=many", "Bearer token", "abcd", valid_body),
    );
    let bad_headers = send(
        port,
        &channel_request("user-1", "limit=25", "no", "abcd", valid_body),
    );
    let bad_cookies = send(
        port,
        &channel_request("user-1", "limit=25", "Bearer token", "ab", valid_body),
    );
    let bad_body = send(
        port,
        &channel_request(
            "user-1",
            "limit=25",
            "Bearer token",
            "abcd",
            r#"{"name":"A","email":"ada@example.com"}"#,
        ),
    );
    let wrong_media_type = send(
        port,
        "POST /users/user-1?limit=25 HTTP/1.1\r\nHost: localhost\r\nContent-Type: text/plain\r\nAuthorization: Bearer token\r\nCookie: session=abcd\r\nContent-Length: 4\r\nConnection: close\r\n\r\nnope",
    );

    let _ = server.kill();
    let output = server.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    fs::remove_dir_all(root).unwrap();

    let accepted = accepted.unwrap_or_else(|| panic!("no response\nstderr:\n{stderr}"));
    assert_eq!(accepted.status, 201, "{}\nstderr:\n{stderr}", accepted.body);
    for expected in [
        r#""id": "user-1""#,
        r#""limit": 25"#,
        r#""verbose": true"#,
        r#""authorization": "Bearer token""#,
        r#""session": "abcd""#,
        r#""name": "Ada""#,
        r#""email": "ada@example.com""#,
    ] {
        assert!(
            accepted.body.contains(expected),
            "missing {expected} in {}",
            accepted.body
        );
    }

    // Each rejection names the channel that failed, which is what tells a
    // client where to look instead of only that the request was refused.
    for (channel, response) in [
        ("params", bad_params),
        ("query", bad_query),
        ("headers", bad_headers),
        ("cookies", bad_cookies),
        ("body", bad_body),
    ] {
        let response = response.unwrap_or_else(|| panic!("no response for {channel}"));
        assert_eq!(response.status, 400, "{channel}: {}", response.body);
        assert!(
            response
                .body
                .contains(&format!("Invalid request {channel}")),
            "{channel} rejection did not name its channel: {}",
            response.body
        );
    }

    let wrong_media_type = wrong_media_type.expect("no response for the media type case");
    assert_eq!(wrong_media_type.status, 415, "{}", wrong_media_type.body);
}

/// An optional request field, narrowed, exposes the members of what it holds.
///
/// Inside `if (request.query.tag)` the value is a `string`, so `.length` is the
/// string's and `.toUpperCase()` is a method it has. Reading the declared type
/// there asked for a member of an optional, and neither expression compiled:
/// the only way to use an optional request field was to copy it out first.
#[test]
fn a_narrowed_optional_request_field_exposes_its_members() {
    let root = project_root("http-narrowed-query");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src").join("main.tyr"),
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpStatus, ServerOptions }} from "tyrus:http";

interface ListQuery {{
    tag?: string;
}}

interface ListResponse {{
    readonly label: string;
    readonly size: usize;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/items", {{
        query: ListQuery,
        response: {{ 200: ListResponse }},
        handler: (request, reply): void => {{
            if (request.query.tag) {{
                reply.status(HttpStatus.Ok).send({{
                    label: request.query.tag.toUpperCase(),
                    size: request.query.tag.length,
                }});
                return;
            }}
            reply.status(HttpStatus.Ok).send({{ label: "none", size: 0 }});
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

    let mut server = Command::new(root.join("target/debug").join("http-narrowed-query"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    let request = |query: &str| {
        format!("GET /items{query} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
    };

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut present = None;
    while Instant::now() < deadline {
        if let Some(response) = send(port, &request("?tag=ada")) {
            present = Some(response);
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let absent = send(port, &request(""));

    let _ = server.kill();
    let _ = server.wait();
    let _ = fs::remove_dir_all(&root);

    let present = present.expect("the server never answered");
    assert_eq!(present.status, 200, "{}", present.body);
    assert!(
        present.body.contains("\"label\": \"ADA\"") && present.body.contains("\"size\": 3"),
        "the narrowed field did not expose its members: {}",
        present.body
    );

    let absent = absent.expect("no response for the absent case");
    assert_eq!(absent.status, 200, "{}", absent.body);
    assert!(
        absent.body.contains("\"label\": \"none\""),
        "the absent branch did not run: {}",
        absent.body
    );
}
