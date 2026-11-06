//! Native end-to-end coverage for typed `catch` dispatch: a thrown value
//! carries the identity of its type, a `catch` takes only what it names, and
//! anything else is passed on to the next enclosing `catch`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct CatchProject {
    root: PathBuf,
    name: String,
}

impl CatchProject {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("typed-catch-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Typed catch integration test" }},
  "security": {{ "envFile": ".env", "envSealed": false }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
        // These fixtures exercise language `throws` and `catch`; adapt filesystem `Result` errors into an explicit user-function throw.
        let source = source
            .replace("readFileSync(", "__catchReadFileSync(")
            .replace("readFile(", "__catchReadFile(");
        let source = if source.contains("__catchReadFileSync(")
            || source.contains("__catchReadFile(")
        {
            let error_import = if source.contains("FsError,") || source.contains("FsError }") {
                ""
            } else {
                "import { FsError } from \"tyrus:fs\";\n"
            };
            format!(
                r#"{error_import}import {{ readFileSync as __nativeReadFileSync, readFile as __nativeReadFile }} from "tyrus:fs";

function __catchReadFileSync(path: string): string throws FsError {{
    const outcome = __nativeReadFileSync(path);
    match (outcome) {{
        Ok(contents) => {{ return contents; }}
        Err(error) => {{ throw error; }}
    }}
}}

async function __catchReadFile(path: string): Promise<string> throws FsError {{
    const outcome = await __nativeReadFile(path);
    match (outcome) {{
        Ok(contents) => {{ return contents; }}
        Err(error) => {{ throw error; }}
    }}
}}

{source}"#
            )
        } else {
            source
        };
        fs::write(root.join("src/main.tyr"), source).unwrap();
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
        Command::new(self.root.join("target/debug").join(&self.name))
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn build_and_run(&self) -> Output {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        self.run()
    }
}

impl Drop for CatchProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_union_catch_reads_the_fields_of_the_variant_that_was_thrown() {
    // The value reached the handler laid out as the union before the value
    // carried its type, so `error.code` read the union's bytes and printed an
    // empty line.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";
import { StreamError } from "tyrus:stream";

const main = (): void => {
    try {
        const text: string = readFileSync("missing-file.txt");
        console.log(text);
    } catch (error: FsError | StreamError) {
        console.log(error.code);
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn a_catch_that_does_not_name_the_thrown_type_passes_it_to_the_enclosing_catch() {
    // `action` throws a `string` the inner `catch (error: Error)` does not
    // handle: the inner `finally` runs and the outer `catch` receives it.
    let project = CatchProject::new(
        r#"function failText(): void throws string {
    throw "from callback";
}

const run = (action: () => void throws string): void throws string => {
    try {
        action();
    } catch (error: Error) {
        console.log("wrong catch", error.message);
    } finally {
        console.log("inner finally");
    }
};

const main = (): void => {
    try {
        run(failText);
    } catch (error: string) {
        console.log("outer", error);
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner finally\nouter from callback\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn thrown_strings_object_literals_and_rethrows_keep_their_values() {
    let project = CatchProject::new(
        r#"function failText(): void throws string {
    throw "text failure";
}

function failError(): void throws Error {
    throw { message: "literal failure", code: "ERR_LITERAL" };
}

const main = (): void => {
    try {
        failText();
    } catch (error: string) {
        console.log(error);
    }
    try {
        failError();
    } catch (error) {
        console.log(error.code, error.message);
    }
    try {
        try {
            throw "inner";
        } catch (inner: string) {
            console.log("caught", inner);
            throw "rethrown";
        } finally {
            console.log("inner finally");
        }
    } catch (outer: string) {
        console.log("outer", outer);
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "text failure\nERR_LITERAL literal failure\ncaught inner\ninner finally\nouter rethrown\n"
    );
}

#[test]
fn an_awaited_union_catch_reads_the_code_of_a_native_rejection() {
    // A native async operation rejects with a code and a message and no value
    // of its own; the union binds the first variant with that shape.
    let project = CatchProject::new(
        r#"import { FsError, readFile } from "tyrus:fs";
import { StreamError } from "tyrus:stream";

const load = async (): Promise<void> => {
    try {
        const text: string = await readFile("missing-file.txt");
        console.log(text);
    } catch (error: FsError | StreamError) {
        console.log(error.code, error.message);
    }
};

const main = async (): Promise<void> => {
    await load();
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_FS_READ_FAILED failed to read file\n"
    );
}

#[test]
fn an_awaited_union_catch_takes_the_variant_the_rejection_code_names() {
    // `StreamError` comes first, and still the fs rejection binds `FsError`:
    // its code names the module it came from.
    let project = CatchProject::new(
        r#"import { FsError, readFile } from "tyrus:fs";
import { StreamError } from "tyrus:stream";

const load = async (): Promise<void> => {
    try {
        const text: string = await readFile("missing-file.txt");
        console.log(text);
    } catch (error: StreamError | FsError) {
        if (error is FsError) {
            console.log("fs", error.code);
        } else {
            console.log("stream", error.code);
        }
    }
};

const main = async (): Promise<void> => {
    await load();
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fs ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn an_awaited_catch_reads_every_field_of_a_thrown_interface() {
    // The rejection carries the thrown value, so fields beyond `code` and
    // `message` reach the handler, and a union binds the variant thrown.
    let project = CatchProject::new(
        r#"interface WriteError {
    code: string;
    message: string;
    path: string;
}

interface QuotaError {
    code: string;
    message: string;
    limit: i32;
}

async function write(path: string): Promise<void> throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full", path: path };
    throw failure;
}

async function store(over: boolean): Promise<void> throws WriteError | QuotaError {
    if (over) {
        const failure: QuotaError = { code: "ERR_QUOTA", message: "quota exceeded", limit: 10 };
        throw failure;
    }
    await write("data.bin");
}

async function main(): Promise<void> {
    try {
        await write("notes.txt");
    } catch (error: WriteError) {
        console.log(error.code, error.path);
    }
    try {
        await store(true);
    } catch (error: WriteError | QuotaError) {
        console.log(error.code, error.message);
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_WRITE notes.txt\nERR_QUOTA quota exceeded\n"
    );
}

#[test]
fn an_outer_catch_handles_what_an_inner_catch_does_not_name() {
    let project = CatchProject::new(
        r#"interface ReadError { code: string; message: string; }
interface WriteError { code: string; message: string; }

function write(): i32 throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "cannot write" };
    throw failure;
}

const main = (): void => {
    try {
        try {
            console.log(write());
        } catch (error: ReadError) {
            console.log("read", error.code);
        } finally {
            console.log("inner finally");
        }
    } catch (error: WriteError) {
        console.log("write", error.code);
    }
    try {
        try {
            throw { code: "ERR_THROWN", message: "thrown" };
        } catch (error: string) {
            console.log("string", error);
        }
    } catch (error: WriteError) {
        console.log("thrown", error.code);
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner finally\nwrite ERR_WRITE\nthrown ERR_THROWN\n"
    );
}

#[test]
fn a_catch_without_an_annotation_takes_native_errors_and_error_subtypes() {
    let project = CatchProject::new(
        r#"import { readFileSync } from "tyrus:fs";

interface WriteError extends Error {
    path: string;
}

function write(): void throws WriteError {
    const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
    throw failure;
}

const main = (): void => {
    try {
        const text: string = readFileSync("missing-file.txt");
        console.log(text);
    } catch (error) {
        console.log(error.code, error.message);
    }
    try {
        write();
    } catch (error) {
        console.log(error.code, error.message);
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_FS_READ_FAILED failed to read file\nERR_WRITE disk is full\n"
    );
}

#[test]
fn is_narrows_a_union_catch_binding_to_the_variant_that_was_thrown() {
    // `is` compares the variant tag, and the branch reads the fields only that
    // variant has. It used to stop the compiler with a panic in `emit_is`.
    let project = CatchProject::new(
        r#"interface ReadError {
    code: string;
    message: string;
}

interface ParseError {
    code: string;
    message: string;
    line: i32;
}

function parse(): void throws ParseError {
    const failure: ParseError = { code: "ERR_PARSE", message: "bad token", line: 7 };
    throw failure;
}

const main = (): void => {
    try {
        parse();
    } catch (error: ReadError | ParseError) {
        console.log(error.code);
        if (error is ParseError) {
            console.log("parse at", error.line);
        } else {
            console.log("read", error.message);
        }
        if (error not is ParseError) {
            console.log("unreachable");
        }
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_PARSE\nparse at 7\n"
    );
}

#[test]
fn is_on_a_caught_error_reads_the_subtype_that_was_thrown() {
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

interface WriteError extends Error {
    path: string;
}

function write(): void throws WriteError {
    const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
    throw failure;
}

const main = (): void => {
    try {
        write();
    } catch (error) {
        if (error is WriteError) {
            console.log("write", error.path);
        } else {
            console.log("other", error.code);
        }
    }
    try {
        const text: string = readFileSync("missing-file.txt");
        console.log(text);
    } catch (error) {
        if (error is WriteError) {
            console.log("write", error.path);
        } else if (error is FsError) {
            console.log("fs", error.code);
        }
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "write a.txt\nfs ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn a_rejected_error_keeps_its_code_for_the_awaiting_catch() {
    let project = CatchProject::new(
        r#"async function main(): Promise<void> {
    try {
        const failure: Error = { message: "rejected on purpose", code: "ERR_REJECTED" };
        await Promise.reject(failure);
    } catch (error) {
        console.log(error.code, error.message);
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_REJECTED rejected on purpose\n"
    );
}

#[test]
fn an_awaiting_catch_does_not_run_for_a_rejection_of_another_type() {
    // `copy` declares `throws ReadError | WriteError`, so its `catch (error:
    // ReadError)` compiles; the `WriteError` it receives fails `copy` instead of
    // running that handler, and the caller's `catch` reads it whole.
    let project = CatchProject::new(
        r#"interface ReadError {
    code: string;
    message: string;
    path: string;
}

interface WriteError {
    code: string;
    message: string;
    path: string;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full", path: "b.txt" };
    throw failure;
}

async function copy(): Promise<void> throws ReadError | WriteError {
    try {
        await write();
    } catch (error: ReadError) {
        console.log("wrong catch", error.code);
    }
    console.log("after try");
}

async function main(): Promise<void> {
    try {
        await copy();
    } catch (error: ReadError | WriteError) {
        if (error is WriteError) {
            console.log("caller", error.code, error.path);
        }
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "caller ERR_WRITE b.txt\n"
    );
}

#[test]
fn is_on_an_error_caught_from_an_awaited_rejection_reads_the_subtype() {
    let project = CatchProject::new(
        r#"import { FsError, readFile } from "tyrus:fs";

interface WriteError extends Error {
    path: string;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
    throw failure;
}

async function main(): Promise<void> {
    try {
        await write();
    } catch (error) {
        console.log(error.code, error.message);
        if (error is WriteError) {
            console.log("write", error.path);
        }
    }
    try {
        const text: string = await readFile("missing-file.txt");
        console.log(text);
    } catch (error) {
        if (error is WriteError) {
            console.log("write", error.path);
        } else if (error is FsError) {
            console.log("fs", error.code);
        }
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_WRITE disk is full\nwrite a.txt\nfs ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn a_rejection_passed_on_by_an_awaiting_catch_runs_its_finally_first() {
    let project = CatchProject::new(
        r#"interface ReadError {
    code: string;
    message: string;
    path: string;
}

interface WriteError {
    code: string;
    message: string;
    path: string;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full", path: "b.txt" };
    throw failure;
}

async function copy(): Promise<void> throws ReadError | WriteError {
    try {
        await write();
    } catch (error: ReadError) {
        console.log("wrong catch", error.code);
    } finally {
        console.log("copy finally");
    }
    console.log("after try");
}

async function main(): Promise<void> {
    try {
        await copy();
    } catch (error: ReadError | WriteError) {
        if (error is WriteError) {
            console.log("caller", error.code, error.path);
        }
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "copy finally\ncaller ERR_WRITE b.txt\n"
    );
}

#[test]
fn is_reads_the_subtype_in_a_catch_that_awaits() {
    // Both handlers suspend before `is`: the kept subtype lives in the async
    // frame, whether it came from a rejection or from a synchronous throw.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

interface WriteError extends Error {
    path: string;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { message: "disk is full", code: "ERR_WRITE", path: "a.txt" };
    throw failure;
}

async function pause(): Promise<void> {}

async function main(): Promise<void> {
    try {
        await write();
    } catch (error) {
        await pause();
        if (error is WriteError) {
            console.log("after await", error.path);
        }
    }
    try {
        const text: string = readFileSync("missing-file.txt");
        console.log(text);
    } catch (error) {
        await pause();
        if (error is FsError) {
            console.log("sync then await", error.code);
        }
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "after await a.txt\nsync then await ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn a_rejection_an_inner_catch_does_not_name_reaches_the_outer_try_of_the_same_function() {
    // The inner `catch` names `ReadError`: its `finally` runs, then the outer
    // `catch` of the same function handles the `WriteError`, the way it would
    // handle a synchronous throw.
    let project = CatchProject::new(
        r#"interface ReadError {
    code: string;
    message: string;
}

interface WriteError {
    code: string;
    message: string;
    path: string;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full", path: "b.txt" };
    throw failure;
}

async function copy(): Promise<void> {
    try {
        try {
            await write();
        } catch (error: ReadError) {
            console.log("wrong catch", error.code);
        } finally {
            console.log("inner finally");
        }
        console.log("after inner try");
    } catch (error: WriteError) {
        console.log("outer", error.code, error.path);
    } finally {
        console.log("outer finally");
    }
    console.log("after outer try");
}

async function main(): Promise<void> {
    await copy();
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner finally\nouter ERR_WRITE b.txt\nouter finally\nafter outer try\n"
    );
}

const NESTED_ERRORS: &str = r#"interface ReadError {
    code: string;
    message: string;
}

interface WriteError {
    code: string;
    message: string;
}

async function pause(): Promise<void> {}

async function read(): Promise<void> throws ReadError {
    const failure: ReadError = { code: "ERR_READ", message: "no such file" };
    throw failure;
}

async function write(): Promise<void> throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full" };
    throw failure;
}
"#;

#[test]
fn a_rejection_passed_on_runs_a_finally_that_awaits_before_leaving_the_function() {
    let project = CatchProject::new(&format!(
        r#"{NESTED_ERRORS}
async function copy(): Promise<void> throws WriteError {{
    try {{
        await write();
    }} catch (error: ReadError) {{
        console.log("wrong catch", error.code);
    }} finally {{
        await pause();
        console.log("finally after await");
    }}
    console.log("after try");
}}

async function main(): Promise<void> {{
    try {{
        await copy();
    }} catch (error: WriteError) {{
        console.log("caller", error.code);
    }}
}}
"#
    ));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "finally after await\ncaller ERR_WRITE\n"
    );
}

#[test]
fn a_rejection_passed_on_through_a_finally_that_awaits_reaches_the_outer_try() {
    let project = CatchProject::new(&format!(
        r#"{NESTED_ERRORS}
async function copy(): Promise<void> {{
    try {{
        try {{
            await write();
        }} catch (error: ReadError) {{
            console.log("wrong catch", error.code);
        }} finally {{
            await pause();
            console.log("inner finally");
        }}
        console.log("after inner try");
    }} catch (error: WriteError) {{
        console.log("outer", error.code);
    }}
    console.log("after outer try");
}}

async function main(): Promise<void> {{
    await copy();
}}
"#
    ));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner finally\nouter ERR_WRITE\nafter outer try\n"
    );
}

#[test]
fn a_rejection_inside_an_awaiting_catch_reaches_the_outer_try() {
    let project = CatchProject::new(&format!(
        r#"{NESTED_ERRORS}
async function copy(): Promise<void> {{
    try {{
        try {{
            await read();
        }} catch (error: ReadError) {{
            console.log("inner", error.code);
            await write();
            console.log("not reached");
        }}
        console.log("after inner try");
    }} catch (error: WriteError) {{
        console.log("outer", error.code);
    }}
    console.log("after outer try");
}}

async function main(): Promise<void> {{
    await copy();
}}
"#
    ));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner ERR_READ\nouter ERR_WRITE\nafter outer try\n"
    );
}

#[test]
fn a_rejection_inside_an_awaiting_finally_reaches_the_outer_try() {
    let project = CatchProject::new(&format!(
        r#"{NESTED_ERRORS}
async function copy(): Promise<void> {{
    try {{
        try {{
            await pause();
            console.log("inner body");
        }} catch (error: ReadError) {{
            console.log("wrong catch", error.code);
        }} finally {{
            await write();
            console.log("not reached");
        }}
        console.log("after inner try");
    }} catch (error: WriteError) {{
        console.log("outer", error.code);
    }}
    console.log("after outer try");
}}

async function main(): Promise<void> {{
    await copy();
}}
"#
    ));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner body\nouter ERR_WRITE\nafter outer try\n"
    );
}

#[test]
fn a_synchronous_throw_passed_on_in_an_async_function_reaches_the_awaiting_catch_whole() {
    // `writeNow` throws synchronously inside a suspending `try` whose `catch`
    // names another type: the value leaves `copy` as its rejection, with every
    // field, the way a `throw` with no `catch` around it would.
    let project = CatchProject::new(
        r#"interface ReadError {
    code: string;
    message: string;
}

interface WriteError {
    code: string;
    message: string;
    path: string;
}

async function pause(): Promise<void> {}

function writeNow(): void throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full", path: "c.txt" };
    throw failure;
}

async function copy(): Promise<void> throws WriteError {
    try {
        writeNow();
        await pause();
    } catch (error: ReadError) {
        console.log("wrong catch", error.code);
    }
    console.log("after try");
}

async function main(): Promise<void> {
    try {
        await copy();
    } catch (error: WriteError) {
        console.log("caller", error.code, error.path);
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "caller ERR_WRITE c.txt\n"
    );
}

#[test]
fn is_names_a_module_error_through_an_import_alias() {
    let project = CatchProject::new(
        r#"import { FsError as FileError, readFileSync } from "tyrus:fs";

const main = (): void => {
    try {
        const text: string = readFileSync("missing-file.txt");
        console.log(text);
    } catch (error) {
        if (error is FileError) {
            console.log("file", error.code);
        } else {
            console.log("other", error.code);
        }
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "file ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn is_on_a_caught_error_works_inside_a_generic_function() {
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

function readOr<T>(fallback: T, path: string): T {
    try {
        const text: string = readFileSync(path);
        console.log(text);
    } catch (error) {
        if (error is FsError) {
            console.log("fs", error.code);
        }
    }
    return fallback;
}

const main = (): void => {
    const count: i32 = readOr<i32>(7, "missing-file.txt");
    const label: string = readOr<string>("none", "missing-file.txt");
    console.log(count, label);
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fs ERR_FS_READ_FAILED\nfs ERR_FS_READ_FAILED\n7 none\n"
    );
}

#[test]
fn typeof_still_tells_a_string_apart_in_a_caught_union() {
    // Typed dispatch reads the tag for `is`; `typeof` keeps answering the
    // general category of the value, as it did before.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

function fail(useText: boolean): void throws string | FsError {
    if (useText) {
        throw "plain text";
    }
    const text: string = readFileSync("missing-file.txt");
    console.log(text);
}

const report = (useText: boolean): void => {
    try {
        fail(useText);
    } catch (error: string | FsError) {
        if (typeof error === "string") {
            console.log("text", error);
        } else {
            console.log("fs", error.code);
        }
    }
};

const main = (): void => {
    report(true);
    report(false);
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "text plain text\nfs ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn every_native_module_error_has_a_typed_recoverable_contract() {
    // Each module failure preserves its error type and stable module code,
    // whether the API exposes it through `throws` or a `Result` value.
    let project = CatchProject::new(
        r#"import { AlgorithmError, bellmanFord } from "tyrus:algorithm";
import { EncodingError, utf8 } from "tyrus:encoding";
import { MemoryError, zeroize } from "tyrus:memory";
import { OsError, trash } from "tyrus:os";
import { ChildProcess, ProcessError, exec } from "tyrus:process";
import { TemporalError, ZonedDateTime, zonedDateTimeFrom } from "tyrus:temporal";

function checkEncoding(): void {
    const invalid: u8[] = [255];
    const outcome: Result<string, EncodingError> = utf8.decode(invalid);
    match (outcome) {
        Ok(_decoded) => { console.log("encoding", false, false); }
        Err(error) => { console.log("encoding", error is EncodingError, error.code.startsWith("ERR_ENCODING_")); }
    }
}

function checkMemory(): void {
    const literal = "api-key";
    const outcome: Result<void, MemoryError> = zeroize(literal);
    match (outcome) {
        Ok() => { console.log("memory", false, false); }
        Err(error) => { console.log("memory", error is MemoryError, error.code.startsWith("ERR_MEMORY_")); }
    }
}

function checkAlgorithm(): void {
    const graph: [usize, i32][][] = [[[1, 1]], [[0, -3]]];
    const outcome: Result<[i32[], Array<usize?>], AlgorithmError> = bellmanFord(graph, 0);
    match (outcome) {
        Ok(_result) => { console.log("algorithm", false, false); }
        Err(error) => { console.log("algorithm", error is AlgorithmError, error.code.startsWith("ERR_ALGORITHM_")); }
    }
}

function checkProcess(): void {
    const outcome: Result<ChildProcess, ProcessError> = exec("tyrus-program-that-does-not-exist");
    match (outcome) {
        Ok(_child) => { console.log("process", false, false); }
        Err(error) => { console.log("process", error is ProcessError, error.code.startsWith("ERR_PROCESS_")); }
    }
}

function checkOs(): void {
    const outcome: Result<void, OsError> = trash("./tyrus-file-that-does-not-exist.txt");
    match (outcome) {
        Ok() => { console.log("os", false, false); }
        Err(error) => { console.log("os", error is OsError, error.code.startsWith("ERR_OS_")); }
    }
}

function checkTemporal(): void {
    const outcome: Result<ZonedDateTime, TemporalError> = zonedDateTimeFrom("not-a-date");
    match (outcome) {
        Ok(_date) => { console.log("temporal", false, false); }
        Err(error) => { console.log("temporal", error is TemporalError, error.code.startsWith("ERR_TEMPORAL_")); }
    }
}

const main = (): void => {
    checkEncoding();
    checkMemory();
    checkAlgorithm();
    checkProcess();
    checkOs();
    checkTemporal();
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "encoding true true\nmemory true true\nalgorithm true true\nprocess true true\nos true true\ntemporal true true\n"
    );
}

#[test]
fn an_awaiting_catch_that_cannot_hold_a_native_rejection_passes_it_on() {
    // `onfail` rethrows the `NetError` Result from a refused connection. The
    // inner `Mismatch` catch cannot take it, so it leaves `connect` and reaches
    // the caller's typed catch.
    let project = CatchProject::new(
        r#"import { NetError, TcpStream, tcpConnect } from "tyrus:net";

interface Mismatch {
    code: string;
    detail: string;
}

async function connect(): Promise<void> throws NetError {
    try {
        const stream: TcpStream = await tcpConnect("127.0.0.1:1", 1) onfail { throw stream; };
        stream.close();
    } catch (error: Mismatch) {
        console.log("wrong catch", error.code);
    }
}

async function main(): Promise<void> {
    try {
        await connect();
    } catch (error: NetError) {
        console.log("caller", error.code.startsWith("ERR_NET_"));
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "caller true\n");
}

#[test]
fn a_synchronous_error_in_a_suspending_switch_reaches_the_catch_around_it() {
    // A `try` whose suspension sits in a `switch` has no jump target of its
    // own: the error thrown after the `await`, in a `case` that does not
    // suspend, and after the `switch` left the function as a rejection and
    // skipped the `catch` written around it.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";
import { OsError } from "tyrus:os";

async function pause(): Promise<void> { return; }

async function afterTheAwait(mode: i32): Promise<void> {
    try {
        switch (mode) {
            case 1: {
                await pause();
                readFileSync("./tyrus-file-that-does-not-exist.txt");
                break;
            }
            case 2: {
                await pause();
                throw "boom".toUpperCase();
            }
            default: {
                readFileSync("./tyrus-file-that-does-not-exist.txt");
                break;
            }
        }
        console.log("unreachable", mode);
    } catch (error: FsError | string) {
        if (error is FsError) {
            console.log("fs", mode, error.code);
        } else {
            console.log("text", mode, error);
        }
    }
}

async function afterTheSwitch(): Promise<void> {
    try {
        switch (true) {
            case true: { await pause(); break; }
        }
        readFileSync("./tyrus-file-that-does-not-exist.txt");
    } catch (error) {
        console.log("after switch", error is FsError, error is OsError, error.code);
    }
}

async function main(): Promise<void> {
    await afterTheAwait(1);
    await afterTheAwait(2);
    await afterTheAwait(3);
    await afterTheSwitch();
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fs 1 ERR_FS_READ_FAILED\ntext 2 BOOM\nfs 3 ERR_FS_READ_FAILED\nafter switch true false ERR_FS_READ_FAILED\n"
    );
}

#[test]
fn a_synchronous_error_in_a_suspending_if_reaches_the_catch_around_it() {
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function read(mode: i32): Promise<void> {
    try {
        if (mode === 1) {
            await pause();
            readFileSync("./tyrus-file-that-does-not-exist.txt");
        }
        readFileSync("./tyrus-other-file-that-does-not-exist.txt");
    } catch (error: FsError) {
        console.log("fs", mode, error.code);
    }
}

async function text(mode: i32): Promise<void> {
    try {
        if (mode === 1) {
            await pause();
            throw "boom".toUpperCase();
        }
        console.log("no text", mode);
    } catch (error: string) {
        console.log("text", mode, error);
    }
}

async function main(): Promise<void> {
    await read(1);
    await read(2);
    await text(1);
    await text(2);
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "fs 1 ERR_FS_READ_FAILED\nfs 2 ERR_FS_READ_FAILED\ntext 1 BOOM\nno text 2\n"
    );
}

#[test]
fn a_synchronous_error_a_suspending_catch_does_not_name_runs_the_finally_once_and_leaves() {
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function run(mode: i32): Promise<void> throws FsError {
    try {
        switch (mode) {
            case 1: {
                await pause();
                readFileSync("./tyrus-file-that-does-not-exist.txt");
                break;
            }
            default: {
                throw "text " + "error";
            }
        }
        console.log("unreachable");
    } catch (error: string) {
        console.log("caught", error);
    } finally {
        console.log("finally", mode);
    }
    console.log("after", mode);
}

async function main(): Promise<void> {
    try {
        await run(2);
        await run(1);
    } catch (error: FsError) {
        console.log("caller", error.code, error.message);
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "caught text error\nfinally 2\nafter 2\nfinally 1\ncaller ERR_FS_READ_FAILED failed to read file\n"
    );
}

#[test]
fn a_throw_inside_the_catch_of_a_nested_suspending_try_reaches_the_outer_catch() {
    // The inner `finally` ran twice (the recovery pushed it again over the copy
    // the site had open), and the value thrown by the inner `catch` failed the
    // function instead of reaching the outer `catch`. Both the synchronous
    // error and the awaited rejection take that path.
    let project = CatchProject::new(
        r#"import { FsError, readFile, readFileSync } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function run(synchronous: boolean): Promise<void> {
    try {
        try {
            if (synchronous) {
                await pause();
                readFileSync("./tyrus-file-that-does-not-exist.txt");
            } else {
                await readFile("./tyrus-file-that-does-not-exist.txt");
            }
            console.log("unreachable");
        } catch (error: FsError) {
            console.log("inner", error.code);
            throw "from inner catch";
        } finally {
            console.log("inner finally");
        }
        console.log("unreachable after inner");
    } catch (error: string) {
        console.log("outer", error);
    }
}

async function main(): Promise<void> {
    await run(true);
    await run(false);
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "inner ERR_FS_READ_FAILED\ninner finally\nouter from inner catch\ninner ERR_FS_READ_FAILED\ninner finally\nouter from inner catch\n"
    );
}

#[test]
fn thrown_interfaces_from_a_suspending_if_reach_the_union_catch_whole() {
    let project = CatchProject::new(
        r#"async function pause(): Promise<void> { return; }

interface Limit {
    name: string;
    maximum: i32;
}

interface Missing {
    key: string;
}

function check(value: i32): void throws Limit | Missing {
    if (value > 10) {
        const limit: Limit = { name: "value", maximum: 10 };
        throw limit;
    }
    if (value < 0) {
        const missing: Missing = { key: "value" };
        throw missing;
    }
}

async function run(value: i32): Promise<void> {
    try {
        if (value !== 5) {
            await pause();
            check(value);
        }
        console.log("ok", value);
    } catch (error: Limit | Missing) {
        if (error is Limit) {
            console.log("limit", error.name, error.maximum);
        } else {
            console.log("missing", error.key);
        }
    }
}

async function main(): Promise<void> {
    await run(20);
    await run(-1);
    await run(5);
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "limit value 10\nmissing value\nok 5\n"
    );
}

#[test]
fn stream_http_and_net_rejections_are_errors_that_is_can_name() {
    let project = CatchProject::new(
        r#"import { HttpError, fetch } from "tyrus:http";
import { NetError, TcpListener, TcpStream, tcpConnect, tcpListen } from "tyrus:net";
import { DuplexStream, StreamError } from "tyrus:stream";

async function checkHttp(): Promise<void> {
    try {
        const response = await fetch.get("http://127.0.0.1:1/");
        console.log("http reached", response.status());
    } catch (error) {
        console.log("http", error is HttpError, error is NetError, error.code.startsWith("ERR_HTTP_"));
    }
}

async function checkNet(): Promise<void> {
    try {
        const stream: TcpStream = await tcpConnect("127.0.0.1:1", 1000) onfail { throw stream; };
        stream.close();
    } catch (error) {
        console.log("net", error is NetError, error is HttpError, error.code);
    }
}

async function checkStream(): Promise<void> {
    try {
        const listener: TcpListener = await tcpListen("127.0.0.1:0", 1000) onfail { throw listener; };
        const client: TcpStream = await tcpConnect(listener.localAddress(), 1000) onfail { throw client; };
        const server: TcpStream = await listener.accept(1000) onfail { throw server; };
        const duplex: DuplexStream<u8[], u8[]> = await client.stream() onfail { throw duplex; };
        const writable = duplex.writable;
        const closing = writable.close();
        await closing;
        await duplex.writable.write([1, 2]);
        console.log("stream wrote after close");
        server.close();
        listener.close();
    } catch (error) {
        console.log("stream", error is StreamError, error is NetError, error.code);
    }
}

async function main(): Promise<void> {
    await checkHttp();
    await checkNet();
    await checkStream();
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut lines = stdout.lines();
    assert_eq!(lines.next(), Some("http true false true"));
    let net = lines.next().expect("expected the TCP connection result");
    assert!(net.starts_with("net true false ERR_NET_"), "{stdout}");
    let stream = lines.next().expect("expected the stream result");
    assert!(
        stream == "stream true false ERR_STREAM_CLOSED"
            || (stream.starts_with("stream false true ERR_NET_")
                && net.ends_with("ERR_NET_PERMISSION")),
        "{stdout}"
    );
    assert!(lines.next().is_none(), "unexpected output: {stdout}");
}

#[test]
fn a_thrown_string_keeps_its_type_through_an_awaited_rejection() {
    // The rejection carried only the text, so a union `catch` of a module
    // error and text read it as the module error, with `ERR_RUNTIME` for a
    // code, and a `catch` naming only the module error took it.
    let project = CatchProject::new(
        r#"import { FsError } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function fail(): Promise<void> throws string {
    await pause();
    throw "boom".toUpperCase();
}

async function onlyModuleErrors(): Promise<void> throws string {
    try {
        await fail();
    } catch (error: FsError) {
        console.log("wrong catch", error.code);
    }
}

async function main(): Promise<void> {
    try {
        await fail();
    } catch (error: FsError | string) {
        if (error is FsError) {
            console.log("fs", error.code);
        } else {
            console.log("text", error);
        }
    }
    try {
        await onlyModuleErrors();
    } catch (error: string) {
        console.log("passed on", error);
    }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "text BOOM\npassed on BOOM\n"
    );
}

#[test]
fn rethrowing_a_caught_error_hands_on_the_subtype_that_arrived() {
    // `throw error;` threw the `Error` projection, so the next `catch` saw an
    // `Error` and `is FsError` answered false.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

interface ConfigError extends Error {
    path: string;
}

function loadConfig(path: string): string throws ConfigError {
    const failure: ConfigError = { message: "config is missing", code: "ERR_CONFIG", path: path };
    throw failure;
}

function readSettings(useConfig: boolean): string throws Error {
    try {
        if (useConfig) {
            return loadConfig("app.json");
        }
        return readFileSync("./tyrus-file-that-does-not-exist.txt");
    } catch (error) {
        if (error is ConfigError) {
            console.log("handled", error.path);
            return "";
        }
        throw error;
    }
}

const main = (): void => {
    try {
        console.log(readSettings(true));
        console.log(readSettings(false));
    } catch (error) {
        if (error is FsError) {
            console.log("caller", error.code, error.message);
        } else {
            console.log("caller lost the subtype", error.code);
        }
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "handled app.json\n\ncaller ERR_FS_READ_FAILED failed to read file\n"
    );
}

#[test]
fn a_catch_around_a_suspending_switch_runs_a_finally_that_awaits() {
    // The recovery ran the `finally` inline after the `catch`, where its
    // `await` had no continuation, and the build stopped with "await codegen
    // is not implemented for continuation value", with or without `is`.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function named(mode: i32): Promise<void> {
    try {
        switch (mode) {
            case 1: {
                await pause();
                readFileSync("./tyrus-file-that-does-not-exist.txt");
                break;
            }
        }
        console.log("named no error", mode);
    } catch (error: FsError) {
        console.log("named", error.code);
    } finally {
        await pause();
        console.log("named finally", mode);
    }
}

async function narrowed(mode: i32): Promise<void> {
    try {
        switch (mode) {
            case 1: {
                await pause();
                readFileSync("./tyrus-file-that-does-not-exist.txt");
                break;
            }
        }
        console.log("narrowed no error", mode);
    } catch (error) {
        if (error is FsError) {
            console.log("narrowed", error.code);
        }
    } finally {
        await pause();
        console.log("narrowed finally", mode);
    }
    console.log("after", mode);
}

async function main(): Promise<void> {
    await named(1);
    await named(2);
    await narrowed(1);
    await narrowed(2);
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "named ERR_FS_READ_FAILED\nnamed finally 1\nnamed no error 2\nnamed finally 2\nnarrowed ERR_FS_READ_FAILED\nnarrowed finally 1\nafter 1\nnarrowed no error 2\nnarrowed finally 2\nafter 2\n"
    );
}

#[test]
fn a_catch_that_awaits_receives_a_synchronous_error_from_a_suspending_try() {
    // The checker refused this shape, since no jump reaches a `catch` that
    // resumes on a later poll call; the guarded poll delivers it as a
    // rejection, in a plain `try`, a suspending `if` and a suspending `switch`.
    let project = CatchProject::new(
        r#"import { FsError, readFileSync } from "tyrus:fs";

async function pause(): Promise<void> { return; }

async function plain(): Promise<void> {
    try {
        await pause();
        readFileSync("./tyrus-file-that-does-not-exist.txt");
    } catch (error) {
        await pause();
        console.log("plain", error is FsError, error.code);
    }
}

async function branch(flag: boolean): Promise<void> {
    try {
        if (flag) {
            await pause();
            readFileSync("./tyrus-file-that-does-not-exist.txt");
        }
        console.log("branch no error");
    } catch (error: FsError) {
        await pause();
        console.log("branch", error.code);
    }
}

async function choose(mode: i32): Promise<void> {
    try {
        switch (mode) {
            case 1: {
                await pause();
                throw "boom".toUpperCase();
            }
        }
        console.log("choose no error");
    } catch (error: string) {
        await pause();
        console.log("choose", error);
    }
}

async function main(): Promise<void> {
    await plain();
    await branch(true);
    await branch(false);
    await choose(1);
    await choose(2);
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "plain true ERR_FS_READ_FAILED\nbranch ERR_FS_READ_FAILED\nbranch no error\nchoose BOOM\nchoose no error\n"
    );
}

#[test]
fn a_synchronous_error_passed_on_through_an_awaiting_finally_runs_it_first() {
    // The checker refused these with `E3299`, since the jump that carries the
    // error cannot wait for a `finally` that awaits. The guarded poll routes it
    // there instead: in a plain suspending `try`, in a suspending `if`, in a
    // nested `try` whose outer `catch` handles the error, and for an error
    // thrown by the `catch` itself.
    let project = CatchProject::new(
        r#"interface ReadError { code: string; message: string; }
interface WriteError { code: string; message: string; }

async function pause(): Promise<void> {}

function writeNow(): void throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE", message: "disk is full" };
    throw failure;
}

function readNow(): void throws ReadError {
    const failure: ReadError = { code: "ERR_READ", message: "missing" };
    throw failure;
}

async function plain(): Promise<void> throws WriteError {
    try {
        await pause();
        writeNow();
    } catch (error: ReadError) {
        console.log("wrong", error.code);
    } finally {
        await pause();
        console.log("plain finally");
    }
}

async function branch(flag: boolean): Promise<void> throws WriteError {
    try {
        if (flag) {
            await pause();
            writeNow();
        }
        console.log("branch no error");
    } catch (error: ReadError) {
        console.log("wrong", error.code);
    } finally {
        await pause();
        console.log("branch finally", flag);
    }
}

async function nested(): Promise<void> {
    try {
        try {
            await pause();
            writeNow();
        } catch (error: ReadError) {
            console.log("wrong", error.code);
        } finally {
            await pause();
            console.log("nested finally");
        }
        console.log("unreachable");
    } catch (error: WriteError) {
        console.log("nested outer", error.code);
    }
}

async function rethrow(): Promise<void> throws string {
    try {
        await pause();
        readNow();
    } catch (error: ReadError) {
        throw "rethrown " + error.code;
    } finally {
        await pause();
        console.log("rethrow finally");
    }
}

async function main(): Promise<void> {
    try { await plain(); } catch (error: WriteError) { console.log("plain caller", error.code); }
    try { await branch(true); } catch (error: WriteError) { console.log("branch caller", error.code); }
    try { await branch(false); } catch (error: WriteError) { console.log("unexpected", error.code); }
    await nested();
    try { await rethrow(); } catch (error: string) { console.log("rethrow caller", error); }
}
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "plain finally\nplain caller ERR_WRITE\nbranch finally true\nbranch caller ERR_WRITE\nbranch no error\nbranch finally false\nnested finally\nnested outer ERR_WRITE\nrethrow finally\nrethrow caller rethrown ERR_READ\n"
    );
}

#[test]
fn a_closure_inside_a_try_runs_and_throws_to_the_enclosing_catch() {
    // A closure's own `return` must neither restore the enclosing `try`
    // context nor run its `finally`; a `throw` from the closure still reaches
    // the enclosing `catch`, and that `finally` runs exactly once.
    let project = CatchProject::new(
        r#"const main = (): void => {
    try {
        const values: i32[] = [1, 2, 3];
        console.log(values.filter((value: i32): boolean => { return value > 1; }).length);
        const check = (value: i32): i32 throws string => {
            if (value > 1) {
                throw "too big";
            }
            return value;
        };
        console.log(check(1));
        console.log(check(5));
    } catch (error: string) {
        console.log("caught", error);
    } finally {
        console.log("finally");
    }
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "2\n1\ncaught too big\nfinally\n"
    );
}
