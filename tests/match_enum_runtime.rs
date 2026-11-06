//! Native end-to-end coverage for `match` over algebraic enums and for the
//! compact `pattern => expression;` arm form.

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

struct MatchProject {
    root: PathBuf,
    name: String,
}

impl MatchProject {
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
        let name = format!("match-enum-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "match integration test" }},
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

impl Drop for MatchProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn single_field_enum_payload_binds_the_stored_value() {
    // A one-field payload is written `Variant(x)`, which parses as a plain
    // binding rather than a one-element tuple. Binding the enum value itself
    // instead of its payload used to compile and silently print `0.0`.
    let project = MatchProject::new(
        r#"
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

enum Code {
    Num(i32),
    Text(string),
    Empty,
}

function main(): void {
    const circle: Shape = Shape.Circle(2.5);
    match (circle) {
        Shape.Circle(r) => { console.log(r); }
        Shape.Rect(w, h) => { console.log(w + h); }
    }
    const text: Code = Code.Text("payload");
    match (text) {
        Code.Num(n) => { console.log(n); }
        Code.Text(t) => { console.log(t); }
        Code.Empty => { console.log("empty"); }
    }
    const number: Code = Code.Num(42);
    match (number) {
        Code.Num(n) => { console.log(n); }
        Code.Text(t) => { console.log(t); }
        Code.Empty => { console.log("empty"); }
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "2.5\npayload\n42\n");
}

#[test]
fn multi_field_enum_payload_binds_every_field() {
    let project = MatchProject::new(
        r#"
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

function main(): void {
    const rect: Shape = Shape.Rect(3.0, 4.0);
    match (rect) {
        Shape.Circle(r) => { console.log(r); }
        Shape.Rect(w, h) => { console.log(w * h); }
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "12.0\n");
}

#[test]
fn a_constructor_call_is_a_valid_match_subject() {
    // The subject does not have to be a variable: `match (Shape.Circle(2.0))`
    // used to fail with "cannot infer type for match expression".
    let project = MatchProject::new(
        r#"
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

function main(): void {
    match (Shape.Circle(2.0)) {
        Shape.Circle(r) => { console.log(r); }
        Shape.Rect(w, h) => { console.log(w + h); }
    }
    match (Shape.Rect(3.0, 4.0)) {
        Shape.Circle(r) => { console.log(r); }
        Shape.Rect(w, h) => { console.log(w + h); }
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "2.0\n7.0\n");
}

#[test]
fn compact_arms_return_their_expression() {
    let project = MatchProject::new(
        r#"
enum Shape {
    Circle(f64),
    Rect(f64, f64),
}

function area(shape: Shape): f64 {
    match (shape) {
        Shape.Circle(r) => 3.0 * r * r;
        Shape.Rect(w, h) => w * h;
    }
    return 0.0;
}

function main(): void {
    console.log(area(Shape.Circle(2.0)));
    console.log(area(Shape.Rect(3.0, 4.0)));
}
"#,
    );
    assert_eq!(project.build_and_run(), "12.0\n12.0\n");
}

#[test]
fn compact_and_block_arms_mix_in_one_match() {
    let project = MatchProject::new(
        r#"
function grade(score: i32): string {
    match (score) {
        90..=100 => "A";
        80..=89 => { return "B"; }
        _ => "F";
    }
    return "?";
}

function main(): void {
    console.log(grade(95));
    console.log(grade(83));
    console.log(grade(10));
}
"#,
    );
    assert_eq!(project.build_and_run(), "A\nB\nF\n");
}

#[test]
fn a_compact_arm_without_its_semicolon_is_a_syntax_error() {
    let project = MatchProject::new(
        r#"
function grade(score: i32): string {
    match (score) {
        90..=100 => "A"
        _ => "F";
    }
    return "?";
}

function main(): void {
    console.log(grade(95));
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("Expected Semicolon"),
        "unexpected diagnostic: {stderr}"
    );
}

#[test]
fn a_block_arm_accepts_an_optional_semicolon() {
    let project = MatchProject::new(
        r#"
function grade(score: i32): string {
    match (score) {
        90..=100 => { return "A"; };
        _ => { return "F"; };
    }
    return "?";
}

function main(): void {
    console.log(grade(95));
    console.log(grade(10));
}
"#,
    );
    assert_eq!(project.build_and_run(), "A\nF\n");
}
