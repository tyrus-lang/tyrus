//! Discriminated unions written by the automatic JSON encoder.
//!
//! A union goes out as the object its live variant already is: no envelope, and
//! the discriminant is a field like any other. That is what lets a reader tell
//! the variants apart, and it is why an indistinguishable union is refused
//! rather than written.

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

struct JsonProject {
    root: PathBuf,
    name: String,
}

impl JsonProject {
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
        let name = format!("json-union-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "json union integration test" }},
  "security": {{ "envFile": ".env", "envSealed": false }},
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

    fn build_and_run(&self) -> String {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = self.run();
        assert!(
            run.status.success(),
            "run failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stdout).into_owned()
    }
}

impl Drop for JsonProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn each_variant_is_written_as_the_object_it_is() {
    let project = JsonProject::new(
        r#"
interface Job {
    id: i32;
    state: { kind: "done"; value: i32 } | { kind: "failed"; reason: string };
}

function main(): void {
    const done: Job = { id: 1, state: { kind: "done", value: 42 } };
    console.log(done.toJson());
    const failed: Job = { id: 2, state: { kind: "failed", reason: "timeout" } };
    console.log(failed.toJson());
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "{\"id\": 1,\"state\": {\"kind\": \"done\",\"value\": 42}}\n{\"id\": 2,\"state\": {\"kind\": \"failed\",\"reason\": \"timeout\"}}\n"
    );
}

#[test]
fn a_union_inside_an_array_is_written_variant_by_variant() {
    let project = JsonProject::new(
        r#"
interface Board {
    jobs: ({ kind: "done"; value: i32 } | { kind: "pending" })[];
}

function main(): void {
    const board: Board = { jobs: [{ kind: "done", value: 7 }, { kind: "pending" }] };
    console.log(board.toJson());
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "{\"jobs\": [{\"kind\": \"done\",\"value\": 7},{\"kind\": \"pending\"}]}\n"
    );
}

#[test]
fn a_union_is_indented_like_any_other_nested_object() {
    let project = JsonProject::new(
        r#"
interface Job {
    id: i32;
    state: { kind: "done"; value: i32 } | { kind: "pending" };
}

function main(): void {
    const done: Job = { id: 1, state: { kind: "done", value: 42 } };
    console.log(done.toJson({ pretty: true }));
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "{\n    \"id\": 1,\n    \"state\": {\n        \"kind\": \"done\",\n        \"value\": 42\n    }\n}\n"
    );
}

#[test]
fn a_union_whose_variants_share_a_discriminant_is_refused() {
    // Two variants declaring `kind: "done"` cannot be told apart by a reader,
    // so writing one would produce JSON that means nothing on its own.
    let project = JsonProject::new(
        r#"
interface Ambiguous {
    state: { kind: "done"; value: i32 } | { kind: "done"; reason: string };
}

function main(): void {
    const value: Ambiguous = { state: { kind: "done", value: 1 } };
    console.log(value.toJson());
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("toJson` is not supported"),
        "unexpected diagnostic: {stderr}"
    );
}

#[test]
fn a_union_reads_back_by_its_discriminant() {
    let project = JsonProject::new(
        r#"
interface Job {
    id: i32;
    state: { kind: "done"; value: i32 } | { kind: "failed"; reason: string };
}

function attempt(text: string): void {
    const decoded = Job.fromJson(text);
    if (decoded) {
        const parsed: Job = decoded;
        console.log(parsed.toJson());
    } else {
        const failure: JsonError = decoded;
        console.log(failure.code, failure.path);
    }
}

function main(): void {
    attempt("{\"id\":1,\"state\":{\"kind\":\"done\",\"value\":42}}");
    attempt("{\"id\":2,\"state\":{\"kind\":\"failed\",\"reason\":\"timeout\"}}");
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "{\"id\": 1,\"state\": {\"kind\": \"done\",\"value\": 42}}\n{\"id\": 2,\"state\": {\"kind\": \"failed\",\"reason\": \"timeout\"}}\n"
    );
}

/// A discriminant naming no declared variant is refused rather than guessed at,
/// and it says so with a code of its own: the field is an object of the right
/// general shape, which is a different complaint from a field of the wrong type.
#[test]
fn an_undeclared_variant_is_refused_with_its_own_code() {
    let project = JsonProject::new(
        r#"
interface Job {
    state: { kind: "done"; value: i32 } | { kind: "failed"; reason: string };
}

function attempt(text: string): void {
    const decoded = Job.fromJson(text);
    if (decoded) {
        console.log("accepted");
    } else {
        const failure: JsonError = decoded;
        console.log(failure.code, failure.path);
    }
}

function main(): void {
    attempt("{\"state\":{\"kind\":\"cancelled\"}}");
    attempt("{\"state\":42}");
    attempt("{\"state\":{\"kind\":\"done\"}}");
    attempt("{}");
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "ERR_JSON_UNKNOWN_VARIANT state\nERR_JSON_TYPE_MISMATCH state\nERR_JSON_TYPE_MISMATCH state\nERR_JSON_MISSING_FIELD state\n"
    );
}
