//! Native end-to-end coverage for lexical `defer` and `errdefer` finalizers.

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

struct DeferProject {
    root: PathBuf,
    name: String,
}

impl DeferProject {
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
        let name = format!("defer-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("tyrus.json"), format!(r#"{{"project":{{"name":"{name}","version":"0.1.0","authors":[],"description":"defer integration test"}},"security":{{"envFile":".env","envSealed":false}},"dependencies":{{}}}}"#)).unwrap();
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

impl Drop for DeferProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn assert_program_output(source: &str, expected: &str) {
    let project = DeferProject::new(source);
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed:\n{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), expected);
}

#[test]
fn defer_runs_in_lifo_order_for_normal_nested_scope_exit_and_return() {
    assert_program_output(
        r#"
function value(): i32 {
    defer console.log("outer-a");
    defer { console.log("outer-b"); }
    if (true) {
        defer console.log("inner");
        console.log("body");
    }
    return 7;
}

function main(): void {
    console.log(value());
}
"#,
        "body\ninner\nouter-b\nouter-a\n7\n",
    );
}

#[test]
fn errdefer_precedes_defer_for_explicit_error_and_question_mark_propagation() {
    assert_program_output(
        r#"
function fail(now: boolean): Result<i32, i32> {
    if (now) { return Err(9); }
    return Ok(2);
}

function work(now: boolean): Result<i32, i32> {
    defer console.log("always-a");
    errdefer console.log("error");
    defer { console.log("always-b"); }
    const value = fail(now)?;
    return Ok(value);
}

function main(): void {
    match (work(false)) { value => { console.log(value); } error => { console.log(error); } }
    match (work(true)) { value => { console.log(value); } error => { console.log(error); } }
}
"#,
        "always-b\nalways-a\n2\nerror\nalways-b\nalways-a\n9\n",
    );
}

#[test]
fn defer_runs_for_break_and_continue_that_leave_a_nested_scope() {
    assert_program_output(
        r#"
function main(): void {
    for (let index: i32 = 0; index < 3; index = index + 1) {
        defer console.log(index);
        if (index === 0) { continue; }
        if (index === 1) { break; }
    }
}
"#,
        "0\n1\n",
    );
}

/// Falling off the end of a function is a normal scope exit, and has to run the
/// finalizers exactly like a `return` does.
///
/// Every other case in this file leaves its function through `return`, `break`,
/// `continue` or error propagation, and all of them worked, which is why this
/// gap survived: the terminator the generator builds for an implicit exit is
/// what makes the block terminated, and `pop_scope` skips a terminated block,
/// so the finalizers were dropped with no diagnostic anywhere. A `defer` that
/// closes a file or releases a lock simply did not run.
///
/// The four shapes are the four places that terminator is built: a plain
/// function, an arrow function, the entry `main`, and a specialized generic,
/// the last one stayed broken after the first three were fixed, because it is
/// emitted by its own path.
#[test]
fn defer_runs_when_a_function_falls_off_its_end_without_an_explicit_return() {
    assert_program_output(
        r#"
function plain(): void {
    defer console.log("plain-deferred");
    console.log("plain-body");
}

function generic<T>(value: T): void {
    defer console.log("generic-deferred");
    console.log("generic-body");
}

const arrow = (): void => {
    defer console.log("arrow-deferred");
    console.log("arrow-body");
};

function lifo(): void {
    defer console.log("lifo-first");
    defer console.log("lifo-second");
    console.log("lifo-body");
}

function main(): void {
    plain();
    generic<i32>(1);
    arrow();
    lifo();
    defer console.log("main-deferred");
    console.log("main-body");
}
"#,
        "plain-body\nplain-deferred\ngeneric-body\ngeneric-deferred\narrow-body\narrow-deferred\nlifo-body\nlifo-second\nlifo-first\nmain-body\nmain-deferred\n",
    );
}

/// A closure's finalizers belong to the closure, not to whoever created it.
///
/// Code generation swaps every per-function state stack when it emits a
/// closure body, locals, owned natives, string-array ownership, and the rest,
/// so the closure cannot disturb the function around it. The deferred-finalizer
/// stack was missing from that list, so a `defer` inside a closure registered
/// itself in the *enclosing* function's scope.
///
/// It stayed invisible for as long as an implicit exit emitted no finalizers at
/// all: the registration went to the wrong place and then nobody ran it. Fixing
/// the exit turned it into a finalizer that ran at the caller's exit instead of
/// the closure's.
///
/// The second case below is the one that shows why this is worth a test rather
/// than a comment. With the caller's stack still visible inside the closure, a
/// `return` in the closure sweeps the *caller's* pending finalizers too, so
/// `outer-deferred` runs once inside `inner()` and again at `outer`'s real
/// exit. A finalizer that releases anything would be releasing it twice.
#[test]
fn a_closure_runs_its_own_finalizers_at_its_own_exit() {
    assert_program_output(
        r#"
const outer = (): void => {
    const inner = (): void => {
        defer console.log("inner-deferred");
        console.log("inner-body");
    };

    inner();
    console.log("outer-after-inner");
    inner();
};

function main(): void {
    outer();
    console.log("done");
}
"#,
        "inner-body\ninner-deferred\nouter-after-inner\ninner-body\ninner-deferred\ndone\n",
    );

    assert_program_output(
        r#"
function outer(): void {
    defer console.log("outer-deferred");

    const inner = (): void => {
        defer console.log("inner-deferred");
        console.log("inner-body");
        return;
    };

    inner();
    console.log("outer-body");
}

function main(): void {
    outer();
    console.log("done");
}
"#,
        "inner-body\ninner-deferred\nouter-body\nouter-deferred\ndone\n",
    );

    // Two levels deep, so the swap has to survive being nested inside itself,
    // and a synchronous callback closure in the same function for good measure.
    assert_program_output(
        r#"
function nested(): void {
    defer console.log("host-deferred");

    const outer = (): void => {
        defer console.log("outer-closure-deferred");

        const inner = (): void => {
            defer console.log("inner-closure-deferred");
            console.log("inner-closure-body");
        };

        inner();
        console.log("outer-closure-body");
    };

    const values: i32[] = [1, 2, 3];
    const doubled: i32[] = values.map((n: i32): i32 => {
        return n * 2;
    });

    outer();
    console.log(doubled[2]);
}

function main(): void {
    nested();
}
"#,
        "inner-closure-body\ninner-closure-deferred\nouter-closure-body\nouter-closure-deferred\n6\nhost-deferred\n",
    );
}
