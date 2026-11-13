//! Native end-to-end coverage for TypeScript-compatible `switch` control flow.

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

struct SwitchProject {
    root: PathBuf,
    name: String,
}

impl SwitchProject {
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
        let name = format!("switch-runtime-{suffix}");
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
    "description": "Switch runtime integration test"
  }},
  "security": {{
    "envFile": ".env",
    "envSealed": false
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
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
            .output()
            .unwrap()
    }
}

impl Drop for SwitchProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn switch_falls_through_and_targets_the_innermost_breakable_construct() {
    let project = SwitchProject::new(
        r#"function main(): void {
    let numericTrace: i32 = 0;
    switch (1) {
        case 1: {
            numericTrace = numericTrace * 10 + 1;
        }
        case 2: {
            numericTrace = numericTrace * 10 + 2;
            break;
        }
        default: {
            numericTrace = 999;
        }
    }
    console.log(numericTrace);

    let stringTrace: i32 = 0;
    switch ("beta") {
        case "alpha": {
            stringTrace = 1;
        }
        case "beta": {
            stringTrace = stringTrace * 10 + 2;
        }
        case "gamma": {
            stringTrace = stringTrace * 10 + 3;
            break;
        }
        default: {
            stringTrace = 999;
        }
    }
    console.log(stringTrace);

    let middleDefaultTrace: i32 = 0;
    switch (99) {
        case 1: {
            middleDefaultTrace = 1;
            break;
        }
        default: {
            middleDefaultTrace = 4;
        }
        case 2: {
            middleDefaultTrace = middleDefaultTrace * 10 + 2;
            break;
        }
    }
    console.log(middleDefaultTrace);

    let completedIterations: i32 = 0;
    let nonMatchingIterations: i32 = 0;
    for (let index: i32 = 0; index < 3; index += 1) {
        switch (index) {
            case 1: {
                break;
            }
            default: {
                nonMatchingIterations += 1;
                break;
            }
        }
        completedIterations += 1;
    }
    console.log(completedIterations);
    console.log(nonMatchingIterations);

    let continuedIterations: i32 = 0;
    for (let index: i32 = 0; index < 3; index += 1) {
        switch (index) {
            case 1: {
                continue;
            }
            default: {
                break;
            }
        }
        continuedIterations += 1;
    }
    console.log(continuedIterations);

    let innerLoopCompleted: i32 = 0;
    switch (1) {
        case 1: {
            for (let index: i32 = 0; index < 3; index += 1) {
                innerLoopCompleted += 1;
                break;
            }
            break;
        }
        default: {
            innerLoopCompleted = 999;
        }
    }
    console.log(innerLoopCompleted);

    switch (1) {
        case 1: {
            try {
                break;
            } catch (error: Error) {
                console.log(error.message);
            } finally {
                console.log("finally");
            }
        }
        default: {
            console.log("unreachable");
        }
    }
    console.log("after");
}

"#,
    );

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
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "12\n23\n42\n3\n2\n2\n1\nfinally\nafter\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn match_keeps_an_owned_union_return_alive_through_every_arm() {
    let project = SwitchProject::new(
        r#"type Result = { tag: "success"; data: string } | { tag: "error"; code: i32; message: string };

function createResult(): Result {
    return { tag: "error", code: 404, message: "Not Found" };
}

function main(): void {
    const result: Result = createResult();
    match (result) {
        { tag: "success", data } => { console.log(data); }
        ({ tag: "error", code, message }) if (code === 404) => { console.log(message); }
        { tag: "error", message } => { console.log(message); }
    }
    match (result) {
        { tag: "success", data } => { console.log(data); }
        ({ tag: "error", code, message }) => { console.log(code); console.log(message); }
    }
}
"#,
    );

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
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Not Found\n404\nNot Found\n"
    );
    assert!(output.stderr.is_empty());
}

/// A field every variant of a union declares is readable on the union itself,
/// whichever variant is live. Each variant may keep it at a different index,
/// so the value has to come from the live variant's own layout, not from the
/// first variant's. Before this, the checker accepted the access on object
/// unions and codegen refused it with "field access requires an object value".
#[test]
fn a_field_shared_by_every_union_variant_reads_from_the_live_variant() {
    let project = SwitchProject::new(
        r#"interface ReadError { code: string; line: i32; message: string; }
interface WriteError { message: string; code: string; }
type Failure = { code: string; retry: boolean } | { attempts: i32; code: string };

function describe(error: ReadError | WriteError): string {
    return error.code + ": " + error.message;
}

function failureCode(failure: Failure): string {
    return failure.code;
}

function main(): void {
    const read: ReadError | WriteError = { code: "ERR_READ", line: 3, message: "cannot read" };
    console.log(describe(read));
    const write: ReadError | WriteError = { message: "cannot write", code: "ERR_WRITE" };
    console.log(describe(write));
    const first: Failure = { code: "E1", retry: true };
    const second: Failure = { attempts: 2, code: "E2" };
    console.log(failureCode(first), failureCode(second));
}
"#,
    );
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
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ERR_READ: cannot read\nERR_WRITE: cannot write\nE1 E2\n"
    );
}
