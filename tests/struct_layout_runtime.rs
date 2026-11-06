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
    "description": "struct layout runtime test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

fn build_and_run(root: &std::path::Path, binary_name: &str) -> std::process::Output {
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
    Command::new(root.join("target/debug").join(binary_name))
        .output()
        .unwrap()
}

/// Every field of a padding-heavy interface, written and read back through
/// every path a field value can take.
///
/// This exists to be the net under any change to how fields are laid out. A
/// layout that reorders fields has to permute at every site that turns a field
/// name into a position, reads, writes, construction, nested structs, arrays
/// of structs, values crossing a function boundary, and a site that is missed
/// does not crash: it reads the wrong field and keeps going. So each field here
/// carries a value that could not be confused with any other, and every one is
/// checked after a round trip.
#[test]
fn every_field_survives_a_round_trip() {
    let root = project_root("struct-layout-round-trip");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Inner {
    tag: u8;
    weight: f64;
}

interface Padded {
    flag: boolean;
    id: i64;
    small: u8;
    amount: f64;
    ok: boolean;
    count: i32;
    label: string;
    inner: Inner;
}

function widen(value: Padded): i64 {
    return value.id + (value.count as i64) + (value.small as i64);
}

function main(): void {
    let row: Padded = {
        flag: true,
        id: 900000000001,
        small: 251,
        amount: 2.5,
        ok: false,
        count: 77,
        label: "edge",
        inner: { tag: 9, weight: 0.5 },
    };

    // Every field, read straight back.
    console.log(row.flag, row.id, row.small, row.amount);
    console.log(row.ok, row.count, row.label);
    console.log(row.inner.tag, row.inner.weight);

    // Through a call, so the value crosses a function boundary by value.
    console.log(widen(row));

    // Written, then read back.
    row.count = 5;
    row.small = 2;
    row.label = "written";
    row.flag = false;
    console.log(row.count, row.small, row.label, row.flag);
    console.log(row.id, row.amount, row.ok);

    // Inside an array, indexed back out.
    const rows: Padded[] = [row];
    console.log(rows[0].id, rows[0].label, rows[0].inner.tag);

    // Printing a struct renders the fields in the order they were declared,
    // whatever order they are stored in.
    console.log(row.inner);

    // Destructuring binds by name, so it has to find each field wherever it
    // is stored.
    const { id, count, label, flag } = row;
    console.log(id, count, label, flag);

    // A functional operation over structs reads fields inside a callback,
    // which is a different emission path from a plain field access.
    const many: Padded[] = [row, row];
    const ids: i64[] = many.map((item) => item.id);
    const labels: string[] = many.map((item) => item.label);
    console.log(ids[0], labels[1]);

    // A struct field read through a nested access inside a callback.
    const tags: u8[] = many.map((item) => item.inner.tag);
    console.log(tags[0]);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "struct-layout-round-trip");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "true 900000000001 251 2.5\n\
         false 77 edge\n\
         9 0.5\n\
         900000000329\n\
         5 2 written false\n\
         900000000001 2.5 false\n\
         900000000001 written 9\n\
         { tag: 9, weight: 0.5 }\n\
         900000000001 5 written false\n\
         900000000001 written\n\
         9\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// The same round trip, through an `async` function.
///
/// An async function is compiled into a state machine whose frame holds the
/// locals across a suspension point, so a struct that lives across an `await`
/// is stored into the frame field by field and read back out field by field,
/// emission paths that a synchronous program never reaches.
#[test]
fn every_field_survives_an_async_round_trip() {
    let root = project_root("struct-layout-async");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Inner {
    tag: u8;
    weight: f64;
}

interface Padded {
    flag: boolean;
    id: i64;
    small: u8;
    amount: f64;
    ok: boolean;
    count: i32;
    label: string;
    inner: Inner;
}

const makeRow = async (): Promise<Padded> => {
    return {
        flag: true,
        id: 900000000001,
        small: 251,
        amount: 2.5,
        ok: false,
        count: 77,
        label: "edge",
        inner: { tag: 9, weight: 0.5 },
    };
};

const widenAsync = async (value: Padded): Promise<i64> => {
    return value.id + (value.count as i64) + (value.small as i64);
};

async function main(): Promise<void> {
    // Returned out of an async function: the struct is built inside the state
    // machine and handed back through its result slot.
    const row: Padded = await makeRow();
    console.log(row.flag, row.id, row.small, row.amount);
    console.log(row.ok, row.count, row.label, row.inner.tag);

    // Passed into an async function: the struct crosses into another frame.
    console.log(await widenAsync(await makeRow()));

    // `row` is read after a suspension point, so it is spilled into the frame
    // and reloaded afterwards.
    const other: i64 = await widenAsync(await makeRow());
    console.log(row.id, row.label, row.count, row.inner.weight);
    console.log(other);

    // A field written between two suspensions, then read back.
    let mutated: Padded = await makeRow();
    mutated.count = 5;
    mutated.label = "written";
    console.log(mutated.count, mutated.label, mutated.id, mutated.small);
    console.log(await widenAsync(mutated));
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "struct-layout-async");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "true 900000000001 251 2.5\n\
         false 77 edge 9\n\
         900000000329\n\
         900000000001 edge 77 0.5\n\
         900000000329\n\
         5 written 900000000001 251\n\
         900000000257\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// The same round trip, out through an HTTP response.
///
/// A response schema is serialized field by field into a JSON body, and a
/// request schema is deserialized field by field back into a struct. Both walk
/// the interface's fields positionally, so both have to agree with however the
/// struct is stored, and a disagreement here is invisible from inside the
/// program: the wrong value simply leaves over the network.
#[test]
fn every_field_survives_an_http_round_trip() {
    let root = project_root("struct-layout-http");
    let Some(port) = test_port() else {
        return;
    };
    fs::write(
        root.join("src/main.tyr"),
        format!(
            r#"import {{ createRouter, HttpCacheOptions, HttpError, HttpReply, HttpReplyOptions, HttpRequest, HttpResponse, HttpRouter, HttpStatus }} from "tyrus:http";

export interface Padded {{
    flag: boolean;
    id: i64;
    small: u32;
    amount: f64;
    ok: boolean;
    count: i32;
    label: string;
}}

export interface TagRequest {{
    tags: string[];
    weights: decimal[];
    metadata: TagMetadata;
    related: TagMetadata[];
}}

export interface TagMetadata {{
    source: string;
    priority: i32;
}}

export interface TagResponse {{
    count: usize;
    weight: decimal;
    tags: string[];
    weights: decimal[];
    metadata: TagMetadata;
    related: TagMetadata[];
    notice?: string[];
}}

export interface BinaryResponse {{
    ok: boolean;
}}

type EmptyRequest = {{ }};

export const paddedHandler = (request: EmptyRequest, reply: HttpReply<Padded>): void => {{
    reply.status(HttpStatus.Ok).send({{
        flag: true,
        id: 900000000001,
        small: 251,
        amount: 2.5,
        ok: false,
        count: 77,
        label: "edge",
    }}, {{ cache: {{ etag: "padded-v1" }} }});
}};

export const tagHandler = (request: {{ body: TagRequest }}, reply: HttpReply<TagResponse>): void => {{
    reply.status(HttpStatus.Ok).send({{
        count: request.body.tags.length,
        weight: request.body.weights[1],
        tags: request.body.tags,
        weights: request.body.weights,
        metadata: request.body.metadata,
        related: request.body.related,
    }});
}};

export function binaryGuard(request: HttpRequest): HttpResponse | true {{
    return {{
        statusCode: HttpStatus.Ok,
        body: [66, 73, 78],
        contentType: "application/octet-stream",
        cookies: "",
    }};
}}

export const ignoredBinaryHandler = (request: {{}}, reply: HttpReply<BinaryResponse>): void => {{
    reply.status(HttpStatus.Ok).send({{ ok: true }});
}};

function main(): void {{
    const app: HttpRouter = createRouter();
    app.get("/padded", {{
        response: {{ 200: Padded }},
        handler: paddedHandler,
    }});
    app.post("/tags", {{
        body: TagRequest,
        response: {{ 200: TagResponse }},
        handler: tagHandler,
    }});
    app.get("/binary", {{
        response: {{ 200: BinaryResponse }},
        preHandler: [binaryGuard],
        handler: ignoredBinaryHandler,
    }});
    try {{
        app.listen({port});
    }} catch (error: HttpError) {{
        console.error(error.code);
    }}
}}
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

    let mut server = Command::new(root.join("target/debug").join("struct-layout-http"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let body = request_body(port);
    let tag_body = post_json_body(
        port,
        "/tags",
        r#"{"tags":["first","second","third"],"weights":[1.25,2.5],"metadata":{"source":"form","priority":2},"related":[{"source":"cache","priority":1},{"source":"import","priority":3}]}"#,
    );
    let binary_response = get_response(port, "/binary");
    let _ = server.kill();
    let server_output = server.wait_with_output().unwrap();
    fs::remove_dir_all(root).unwrap();

    // JSON object members have no semantic order, so compare the decoded
    // fields instead of depending on the serializer's key order.
    let body = body.expect("the response body should arrive");
    let actual: serde_json::Value =
        serde_json::from_str(&body).expect("the response body should contain JSON");
    assert_eq!(
        actual,
        serde_json::json!({
            "small": 251,
            "ok": false,
            "label": "edge",
            "id": 900000000001_i64,
            "flag": true,
            "count": 77,
            "amount": 2.5
        }),
        "unexpected response body\nstderr:\n{}",
        String::from_utf8_lossy(&server_output.stderr)
    );
    let tag_body = tag_body.expect("tag response should arrive");
    let tag_json: serde_json::Value =
        serde_json::from_str(&tag_body).expect("the tag response should contain JSON");
    assert_eq!(tag_json["count"], 3, "unexpected tag response: {tag_body}");
    assert_eq!(
        tag_json["tags"],
        serde_json::json!(["first", "second", "third"]),
        "unexpected tag response: {tag_body}"
    );
    // A `decimal` crosses the wire in the same notation it prints in.
    assert_eq!(
        tag_json["weight"], 2.5,
        "unexpected tag response: {tag_body}"
    );
    assert_eq!(
        tag_json["weights"],
        serde_json::json!([1.25, 2.5]),
        "unexpected tag response: {tag_body}"
    );
    assert_eq!(
        tag_json["metadata"],
        serde_json::json!({ "priority": 2, "source": "form" }),
        "unexpected tag response: {tag_body}"
    );
    assert_eq!(
        tag_json["related"],
        serde_json::json!([
            { "priority": 1, "source": "cache" },
            { "priority": 3, "source": "import" }
        ]),
        "unexpected tag response: {tag_body}"
    );
    let binary_response = binary_response.expect("binary response should arrive");
    assert!(
        binary_response.starts_with("HTTP/1.1 200"),
        "{binary_response}"
    );
    assert!(
        binary_response
            .to_ascii_lowercase()
            .contains("content-type: application/octet-stream"),
        "{binary_response}"
    );
    assert!(
        binary_response.ends_with("\r\n\r\nBIN"),
        "{binary_response}"
    );
}

/// `GET /padded` against a server that is still starting up, retried until it
/// answers. Returns the response body, or `None` if it never answered.
fn request_body(port: u16) -> Option<String> {
    get_response(port, "/padded")?
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.trim().to_string())
}

fn get_response(port: u16, path: &str) -> Option<String> {
    use std::io::{Read, Write};

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        if stream
            .write_all(
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .is_err()
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let mut response = String::new();
        if stream.read_to_string(&mut response).is_err() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        return Some(response);
    }
    None
}

fn post_json_body(port: u16, path: &str, body: &str) -> Option<String> {
    use std::io::{Read, Write};

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        let Ok(mut stream) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        };
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        if stream.write_all(request.as_bytes()).is_err() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        let mut response = String::new();
        if stream.read_to_string(&mut response).is_err() {
            std::thread::sleep(std::time::Duration::from_millis(50));
            continue;
        }
        return response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.trim().to_string());
    }
    None
}
