//! What `start` and `close` promise, checked against a real client.
//!
//! The unit tests never bind a socket and the semantic tests stop at the type
//! checker, so neither can say whether a closed server actually stops
//! answering, nor whether a request already running when `close` is called is
//! allowed to finish. Both are the point of the handle, so both are checked
//! here through a socket.

use std::fs;
use std::io::{BufRead, Read, Write};
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
    "description": "http server lifecycle test"
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

/// Sends one request, retrying only the connect: a server that has not bound
/// yet is the normal state right after spawn, while a refused connection after
/// the close is the result the test is looking for.
fn send_once(port: u16, request: &str, read_timeout: Duration) -> Option<Response> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(read_timeout)).unwrap();
    stream.write_all(request.as_bytes()).ok()?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw).ok()?;
    let status = raw
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())?;
    let body = raw
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.trim().to_string())
        .unwrap_or_default();
    Some(Response { status, body })
}

fn wait_until_serving(port: u16, request: &str) -> Response {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(response) = send_once(port, request, Duration::from_secs(10)) {
            return response;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("the server never answered on port {port}");
}

fn get(path: &str) -> String {
    format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
}

fn build_project(root: &std::path::Path, source: String) {
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

#[test]
fn a_started_server_answers_and_a_closed_one_stops_answering() {
    let root = project_root("http-server-lifecycle");
    if test_port().is_none() {
        return;
    }
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Health {{
    readonly ok: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/health", {{
        response: {{ 200: Health }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ ok: true }});
        }},
    }});
    return app.listen(0, {{ host: "127.0.0.1", shutdownTimeout: "10s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    printError("port", server.port());
    await sleepAsync(3000);
    printError("closing");
    await server.close();
    printError("closed");
    // Stays alive well past the close, so the test observes a stopped server
    // rather than a stopped process: an exiting program closes its port too,
    // and an earlier version of this test could not tell the two apart.
    await sleepAsync(15000);
}};
"#
        ),
    );

    let mut child = Command::new(root.join("target/debug").join("http-server-lifecycle"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = child.stderr.take().unwrap();
    let (output_sender, output_receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in std::io::BufReader::new(stderr).lines() {
            if output_sender.send(line).is_err() {
                break;
            }
        }
    });
    let mut server = ServerProcess::new(child);

    let port_line = output_receiver
        .recv_timeout(Duration::from_secs(20))
        .expect("the server did not report its bound port")
        .expect("could not read the server's bound port");
    let port = port_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .expect("the server reported an invalid bound port");

    let serving = wait_until_serving(port, &get("/health"));
    assert_eq!(serving.status, 200, "body: {}", serving.body);
    assert_eq!(serving.body, r#"{"ok": true}"#);

    // Wait for the generated program to finish closing before probing the
    // listener. A fixed delay can expire before a busy runner resumes its task.
    let mut progress = Vec::new();
    let mut closed = false;
    let close_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < close_deadline {
        match output_receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(line)) if line == "closed" => {
                closed = true;
                break;
            }
            Ok(Ok(line)) => progress.push(line),
            Ok(Err(error)) => panic!("could not read server progress: {error}"),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    assert!(
        closed,
        "the generated server did not report that close completed; stderr: {progress:?}"
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    // Some Windows runners time out when connecting to a closed loopback port
    // instead of returning ConnectionRefused. Both outcomes mean no TCP
    // handshake completed; a successful connection means the listener remains.
    let mut stopped = false;
    let mut last_connect = String::new();
    while Instant::now() < deadline {
        match std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_secs(1),
        ) {
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionRefused | std::io::ErrorKind::TimedOut
                ) =>
            {
                stopped = true;
                break;
            }
            Ok(_) => last_connect = "connected".to_string(),
            Err(error) => last_connect = format!("{error:?}"),
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    // The program must still be running: a failed connection after it exited
    // would say nothing about `close`.
    let still_running = server.still_running();
    drop(server);
    let _ = fs::remove_dir_all(&root);
    assert!(
        still_running,
        "the program exited before the port could be observed"
    );
    assert!(
        stopped,
        "the server completed a TCP handshake after `close`; last connect: {last_connect}"
    );
}

#[test]
fn close_waits_for_a_request_that_is_already_running() {
    let root = project_root("http-server-drain");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync(3000);
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ host: "127.0.0.1" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    // Short enough that the slow request below is still running when the
    // close arrives: that overlap is the whole point of the test.
    await sleepAsync(1200);
    await server.close();
    // Leaving immediately makes the process lifetime the observable form of
    // "the close resolved": a close that did not wait would exit here with the
    // slow request still open, and the client would see the connection cut.
    process.exit(0);
}};
"#
        ),
    );

    let server = Command::new(root.join("target/debug").join("http-server-drain"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    // Wait for the server to be up using the fast route, so the slow request
    // below is the first thing in flight rather than a connect retry.
    let ready = wait_until_serving(port, &get("/ready"));
    assert_eq!(ready.status, 200);

    // This request is still running when `close` is called at 1.2s. The
    // handler takes three seconds, so a close that waits for it cannot resolve
    // before then, and one that does not resolves right at the signal.
    let slow = send_once(port, &get("/slow"), Duration::from_secs(20));
    drop(server);
    let _ = fs::remove_dir_all(&root);

    let slow = slow.expect("the in-flight request was dropped by the shutdown");
    assert_eq!(slow.status, 200, "body: {}", slow.body);
    assert_eq!(slow.body, r#"{"done": true}"#);
}

#[test]
fn the_shutdown_deadline_bounds_how_long_close_waits() {
    let root = project_root("http-server-deadline");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync(6000);
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ host: "127.0.0.1", shutdownTimeout: "1s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    await sleepAsync(1200);
    await server.close();
    process.exit(0);
}};
"#
        ),
    );

    let server = Command::new(root.join("target/debug").join("http-server-deadline"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    let ready = wait_until_serving(port, &get("/ready"));
    assert_eq!(ready.status, 200);

    // The handler takes six seconds and the deadline is one, so the close
    // stops waiting around 2.2s in and the program leaves with this request
    // still open. Without the deadline the close would wait it out and the
    // client would be answered.
    let started = Instant::now();
    let slow = send_once(port, &get("/slow"), Duration::from_secs(20));
    let elapsed = started.elapsed();
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(
        slow.is_none(),
        "the deadline did not stop the wait: the request was answered in full"
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "the connection stayed open for {elapsed:?}, past the deadline"
    );
}

/// Past the deadline the requests still running are ended, not merely stopped
/// being waited for.
///
/// The distinction is invisible in the test above, where the program exits
/// right after `close` and takes every connection with it. Here the program
/// stays alive for seconds afterwards, so a connection that outlives the
/// deadline can only be closed by the deadline itself.
#[test]
fn the_shutdown_deadline_ends_the_requests_that_outlast_it() {
    let root = project_root("http-server-deadline-cut");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync(20000);
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ host: "127.0.0.1", shutdownTimeout: "1s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    await sleepAsync(1200);
    await server.close();
    // The process outlives the close by a wide margin, so nothing but the
    // deadline can be what closed the connection.
    await sleepAsync(12000);
    process.exit(0);
}};
"#
        ),
    );

    let server = Command::new(root.join("target/debug").join("http-server-deadline-cut"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    let ready = wait_until_serving(port, &get("/ready"));
    assert_eq!(ready.status, 200);

    // The handler takes twenty seconds and the deadline is one, so the cut
    // lands around 2.2s in. The process is still running for ten seconds after
    // that, and the handler for eighteen.
    let started = Instant::now();
    let slow = send_once(port, &get("/slow"), Duration::from_secs(30));
    let elapsed = started.elapsed();
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert!(
        slow.is_none(),
        "the request outlasted the deadline and was answered in full"
    );
    assert!(
        elapsed < Duration::from_secs(8),
        "the connection was left open for {elapsed:?}, so the deadline did not end it"
    );
}

/// Options that do not write `host` bind every interface, as no options at all
/// would.
///
/// The field exists in the struct whether or not the program wrote it, so
/// "absent" arrives as an empty string. Reading that as an address made
/// `app.listen(port, { requestTimeout: "30s" })` refuse to start, complaining
/// about an address the program had never written.
#[test]
fn options_without_a_host_bind_every_interface() {
    let root = project_root("http-server-no-host");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Ready {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/ready", {{
        response: {{ 200: Ready }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ requestTimeout: "30s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    await sleepAsync(20000);
    await server.close();
}};
"#
        ),
    );

    let server = Command::new(root.join("target/debug").join("http-server-no-host"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    let ready = wait_until_serving(port, &get("/ready"));
    drop(server);
    let _ = fs::remove_dir_all(&root);

    assert_eq!(ready.status, 200);
}

/// A close that had to cut live requests reports it, with a code the program
/// can compare against.
///
/// Without this the deadline is invisible to the program: the only thing that
/// noticed was a client whose connection died, and the process that decided to
/// shut down cannot tell a graceful stop from a forced one.
#[test]
fn a_shutdown_that_missed_its_deadline_rejects_with_a_stable_code() {
    let root = project_root("http-server-deadline-error");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync(20000);
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ host: "127.0.0.1", shutdownTimeout: "1s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    // Long enough for the test to get its slow request in flight first.
    await sleepAsync(4000);
    try {{
        await server.close();
        console.log("closed cleanly");
    }} catch (error: HttpError) {{
        console.log(error.code);
        console.log(error.message);
    }}
    process.exit(0);
}};
"#
        ),
    );

    let server = Command::new(root.join("target/debug").join("http-server-deadline-error"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let server = ServerProcess::new(server);

    let ready = wait_until_serving(port, &get("/ready"));
    assert_eq!(ready.status, 200);

    // Held open in a thread of its own: the close has to find this request
    // running, and the deadline has to be what ends it.
    let slow = std::thread::spawn(move || send_once(port, &get("/slow"), Duration::from_secs(30)));

    let output = server.into_output();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let _ = slow.join();
    let _ = fs::remove_dir_all(&root);

    assert!(
        stdout.contains("ERR_HTTP_SHUTDOWN_TIMEOUT"),
        "the forced shutdown was not reported to the program:\n{stdout}"
    );
    assert!(
        !stdout.contains("closed cleanly"),
        "a shutdown that cut live requests reported itself as clean:\n{stdout}"
    );
}

/// What a program that does not catch the shutdown failure sees.
///
/// Encoded as a test because it is the one behaviour change that reaches
/// programs written before `close` could fail, and it should not be possible to
/// change it without noticing.
#[test]
fn an_uncaught_shutdown_failure_stops_the_program() {
    let root = project_root("http-server-uncaught-shutdown");
    let Some(port) = test_port() else {
        return;
    };
    build_project(
        &root,
        format!(
            r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const startServer = (): Result<HttpServer, HttpError> => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync(20000);
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    return app.listen({port}, {{ host: "127.0.0.1", shutdownTimeout: "1s" }});
}};

const main = async (): Promise<void> => {{
    const server = startServer() onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    await sleepAsync(4000);
    await server.close();
    console.log("after close");
    process.exit(0);
}};
"#
        ),
    );

    let server = Command::new(
        root.join("target/debug")
            .join("http-server-uncaught-shutdown"),
    )
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .unwrap();
    let server = ServerProcess::new(server);

    let ready = wait_until_serving(port, &get("/ready"));
    assert_eq!(ready.status, 200);
    let slow = std::thread::spawn(move || send_once(port, &get("/slow"), Duration::from_secs(30)));

    let output = server.into_output();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let status = output.status;
    let _ = slow.join();
    let _ = fs::remove_dir_all(&root);

    assert!(
        !status.success(),
        "a shutdown that cut live requests left the process claiming success"
    );
    assert!(
        !stdout.contains("after close"),
        "the program carried on past a `close` that failed:\n{stdout}"
    );
    // The stable code, not only the prose: this line is all that is left of an
    // error nobody caught, and the code is the half a log search can match.
    assert!(
        stderr.contains("ERR_HTTP_SHUTDOWN_TIMEOUT"),
        "the failure reached stderr without its stable code:\n{stderr}"
    );
}

/// A server process that is killed however the test ends.
///
/// A test that panics between spawning and killing leaves the server running
/// and holding its port. Nothing notices at the time: the failure lands on
/// whatever test runs next and finds an address in use, or worse, gets an
/// answer from a server it did not start.
struct ServerProcess(Option<std::process::Child>);

impl ServerProcess {
    fn new(child: std::process::Child) -> Self {
        Self(Some(child))
    }

    /// Whether the process is still running, without waiting for it.
    fn still_running(&mut self) -> bool {
        self.0
            .as_mut()
            .expect("the child is only taken at the end")
            .try_wait()
            .unwrap()
            .is_none()
    }

    /// Waits for the process to end on its own and takes its output.
    fn into_output(mut self) -> std::process::Output {
        self.0
            .take()
            .expect("the child is taken exactly once")
            .wait_with_output()
            .unwrap()
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn signal_server_source(port: u16, slow_millis: u64) -> String {
    format!(
        r#"
import {{ createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions }} from "tyrus:http";
import {{ sleepAsync }} from "tyrus:task";

interface Slow {{
    readonly done: boolean;
}}

const main = (): void => {{
    const app: HttpRouter = createRouter();
    app.get("/slow", {{
        response: {{ 200: Slow }},
        handler: async (request, reply): Promise<void> => {{
            await sleepAsync({slow_millis});
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    app.get("/ready", {{
        response: {{ 200: Slow }},
        handler: (request, reply): void => {{
            reply.status(HttpStatus.Ok).send({{ done: true }});
        }},
    }});
    const server = app.listen({port}, {{ host: "127.0.0.1" }}) onfail {{
        printError(`${{server.code}}: ${{server.message}}`);
        return;
    }};
    console.log("after listen", server.port());
}};
"#
    )
}

#[cfg(unix)]
fn send_signal(child: &std::process::Child, signal: &str) {
    let status = Command::new("kill")
        .args([format!("-{signal}"), child.id().to_string()])
        .status()
        .unwrap();
    assert!(status.success(), "kill -{signal} failed");
}

#[cfg(any(unix, windows))]
fn wait_for_exit(server: &mut ServerProcess, within: Duration) -> std::process::ExitStatus {
    let deadline = Instant::now() + within;
    loop {
        if let Some(status) = server
            .0
            .as_mut()
            .expect("the child is only taken at the end")
            .try_wait()
            .unwrap()
        {
            return status;
        }
        assert!(
            Instant::now() < deadline,
            "the server did not exit after the signal"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(windows)]
fn send_ctrl_break(child: &std::process::Child) {
    let sent = unsafe {
        windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
            windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
            child.id(),
        )
    } != 0;
    assert!(sent, "GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT) failed");
}

/// A SIGTERM closes a `listen` server the way `close()` does: the request
/// already running is answered, and the process ends with `0` once it has,
/// after running the code written after `listen`.
#[cfg(unix)]
#[test]
fn a_termination_signal_drains_the_server_and_ends_the_process() {
    let root = project_root("http-server-sigterm");
    let Some(port) = test_port() else {
        return;
    };
    build_project(&root, signal_server_source(port, 1500));
    let child = Command::new(root.join("target/debug").join("http-server-sigterm"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = ServerProcess::new(child);
    assert_eq!(wait_until_serving(port, &get("/ready")).status, 200);

    let slow = std::thread::spawn(move || send_once(port, &get("/slow"), Duration::from_secs(20)));
    std::thread::sleep(Duration::from_millis(400));
    send_signal(server.0.as_ref().unwrap(), "TERM");

    let slow = slow.join().unwrap();
    let status = wait_for_exit(&mut server, Duration::from_secs(20));
    let output = server.into_output();
    let _ = fs::remove_dir_all(&root);

    let slow = slow.unwrap_or_else(|| {
        panic!(
            "the request in flight was dropped by the signal; status: {status:?}; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(slow.status, 200, "body: {}", slow.body);
    assert_eq!(status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("after listen"), "stdout: {stdout}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Received SIGTERM, shutting down"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn ctrl_break_drains_the_http_server_and_ends_the_process() {
    let root = project_root("http-server-ctrl-break");
    let Some(port) = test_port() else {
        return;
    };
    build_project(&root, signal_server_source(port, 1500));
    let executable = root
        .join("target/debug")
        .join("http-server-ctrl-break")
        .with_extension("exe");
    use std::os::windows::process::CommandExt;
    let child = Command::new(executable)
        .creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = ServerProcess::new(child);
    assert_eq!(wait_until_serving(port, &get("/ready")).status, 200);

    let slow = std::thread::spawn(move || send_once(port, &get("/slow"), Duration::from_secs(20)));
    std::thread::sleep(Duration::from_millis(400));
    send_ctrl_break(server.0.as_ref().unwrap());

    let slow = slow.join().unwrap();
    let status = wait_for_exit(&mut server, Duration::from_secs(20));
    let output = server.into_output();
    let _ = fs::remove_dir_all(&root);

    let slow = slow.unwrap_or_else(|| {
        panic!(
            "the request in flight was dropped by Ctrl+Break; status: {status:?}; stdout: {}; stderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(slow.status, 200, "body: {}", slow.body);
    assert_eq!(status.code(), Some(0));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("after listen"),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Received Ctrl+Break, shutting down"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A second signal does not wait for the drain.
#[cfg(unix)]
#[test]
fn a_second_interrupt_ends_the_process_without_waiting() {
    let root = project_root("http-server-sigint-twice");
    let Some(port) = test_port() else {
        return;
    };
    build_project(&root, signal_server_source(port, 30_000));
    let child = Command::new(root.join("target/debug").join("http-server-sigint-twice"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut server = ServerProcess::new(child);
    assert_eq!(wait_until_serving(port, &get("/ready")).status, 200);

    let _slow = std::thread::spawn(move || send_once(port, &get("/slow"), Duration::from_secs(40)));
    std::thread::sleep(Duration::from_millis(400));
    send_signal(server.0.as_ref().unwrap(), "INT");
    std::thread::sleep(Duration::from_millis(400));
    send_signal(server.0.as_ref().unwrap(), "INT");

    let started = Instant::now();
    let status = wait_for_exit(&mut server, Duration::from_secs(10));
    let _ = fs::remove_dir_all(&root);
    assert_eq!(status.code(), Some(130));
    assert!(started.elapsed() < Duration::from_secs(5));
}
