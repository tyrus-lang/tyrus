//! Native end-to-end coverage for the `loop` keyword, `break`/`continue`
//! transfers, and `break <value>` loop expressions.

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

struct LoopProject {
    root: PathBuf,
    name: String,
}

impl LoopProject {
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
        let name = format!("loop-runtime-{suffix}");
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
    "description": "Loop runtime integration test"
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

impl Drop for LoopProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn loop_statement_transfers_break_and_continue() {
    let project = LoopProject::new(
        r#"function main(): void {
    let counter: i32 = 0;

    loop {
        counter += 1;
        if (counter < 3) { continue; }
        break;
    }
    console.log(counter);

    let sumEven: i32 = 0;
    let n: i32 = 0;

    loop {
        n += 1;
        if (n > 6) { break; }
        if (n % 2 === 1) { continue; }
        sumEven += n;
    }
    console.log(sumEven);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "3\n12\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn loop_expression_produces_break_values() {
    let project = LoopProject::new(
        r#"function main(): void {
    const found: i32 = loop {
        break 7;
    };
    console.log(found);

    const label: string = loop {
        break "ready";
    };
    console.log(label);

    const nested: i32 = loop {
        const inner: i32 = loop {
            break 10;
        };
        if (inner === 10) {
            break inner + 1;
        }
        break 0;
    };
    console.log(nested);

    let index: i32 = 0;
    const fromBody: i32 = loop {
        index += 1;
        if (index < 4) { continue; }
        break index * 2;
    };
    console.log(fromBody);
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
        "7\nready\n11\n8\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn break_with_a_value_is_rejected_outside_a_loop_expression() {
    let project = LoopProject::new(
        r#"function main(): void {
    let index: i32 = 0;
    while (index < 3) {
        index += 1;
        break 1;
    }
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build unexpectedly succeeded:\nstdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(stderr.contains("E0571"), "expected E0571, got:\n{stderr}");
}

#[test]
fn loop_break_values_must_agree_on_type() {
    let project = LoopProject::new(
        r#"function main(): void {
    const value = loop {
        if (true) { break 42; }
        break "text";
    };
    console.log(value);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build unexpectedly succeeded:\nstdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(stderr.contains("E0308"), "expected E0308, got:\n{stderr}");
}

#[test]
fn loop_without_a_break_value_is_void_in_expression_position() {
    let project = LoopProject::new(
        r#"function main(): void {
    const value: i32 = loop {
        break;
    };
    console.log(value);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build unexpectedly succeeded:\nstdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(stderr.contains("E0308"), "expected E0308, got:\n{stderr}");
}

/// A `loop` with no `break` targeting it never falls through, so a `return`
/// inside it satisfies the function's return type, exactly as the
/// `while (true)` spelling of the same function already did.
///
/// The two disagreed: `while (true) { return 7; }` compiled and
/// `loop { return 7; }` failed E0310, because the return analysis knew the
/// `while (true)` shape and had no arm for `loop`. That is a difference the
/// language does not admit anywhere else, `loop` is specified as equivalent to
/// `while (true)`, so the divergence was a bug in the analysis, not a rule.
#[test]
fn a_loop_without_a_break_returns_on_every_path() {
    let project = LoopProject::new(
        r#"function fromLoop(): i32 {
    loop {
        return 7;
    }
}

function fromWhile(): i32 {
    while (true) {
        return 9;
    }
}

function nestedBreakDoesNotCount(): i32 {
    loop {
        loop {
            break;
        }
        return 5;
    }
}

function main(): void {
    console.log(fromLoop());
    console.log(fromWhile());
    console.log(nestedBreakDoesNotCount());
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
        "program failed:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "7\n9\n5\n");
}

/// A `loop` that *can* fall through still needs a return after it: the break
/// makes the loop terminate, so the function does not return on every path.
/// This is the other half of the arm above, without it, the fix could have
/// been "treat every `loop` as divergent", which would let a real missing
/// return through.
#[test]
fn a_loop_with_a_break_still_requires_a_return() {
    let project = LoopProject::new(
        r#"function maybe(): i32 {
    loop {
        break;
    }
}

function main(): void {
    console.log(maybe());
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "build unexpectedly succeeded:\nstdout:\n{}",
        String::from_utf8_lossy(&build.stdout)
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(stderr.contains("E0310"), "expected E0310, got:\n{stderr}");
}

/// A binding declared inside a hot loop must not consume stack per iteration.
///
/// `alloca` runs every time control reaches it and its memory comes back only
/// when the function returns, so a slot reserved in a loop body grew the frame
/// once per iteration: a million-iteration `for` with a single `const` in it
/// died of SIGSEGV, having exhausted the stack. Both spellings are covered
/// because the `loop` expression reserved a slot of its own on top of the
/// binding's, and each had to move to the entry block independently.
///
/// A million iterations is the point the old code died at; the assertion is
/// that the program finishes and prints the right sum, not how fast.
#[test]
fn a_binding_in_a_hot_loop_does_not_grow_the_stack() {
    let project = LoopProject::new(
        r#"function main(): void {
    let plain: i64 = 0;
    for (let i: i32 = 0; i < 1000000; i += 1) {
        const value: i32 = i;
        plain += value as i64;
    }
    console.log(plain);

    let fromLoop: i64 = 0;
    for (let i: i32 = 0; i < 1000000; i += 1) {
        const value: i32 = loop {
            break i;
        };
        fromLoop += value as i64;
    }
    console.log(fromLoop);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed (a stack overflow shows up here as a signal):\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "499999500000\n499999500000\n"
    );
}

/// The code generation half of the counter inference: the checker never writes
/// the inferred type back into the AST, so `emit_for` has to reach the same
/// answer on its own. A disagreement is not a diagnostic, it is an `i32`
/// counter compared against a `usize` bound in the emitted IR, so only running
/// the program proves the two sides agree.
#[test]
fn for_counter_takes_its_type_from_the_loop_bound() {
    let project = LoopProject::new(
        r#"function takes(n: i32): void {
    console.log(n);
}

function main(): void {
    const nums: i32[] = [10, 20, 30];

    // No annotation: the `usize` bound decides, so `nums[i]` indexes cleanly.
    for (let i = 0; i < nums.length; i++) {
        console.log(nums[i]);
    }

    // An arithmetic bound keeps the bound's type.
    for (let i = 0; i < nums.length - 1; i++) {
        console.log(nums[i]);
    }

    // A signed bound leaves the counter `i32`, so a negative intermediate
    // value and a call expecting `i32` both still work.
    for (let i = 0; i < 3; i++) {
        takes(i - 5);
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
        "10\n20\n30\n10\n20\n-5\n-4\n-3\n"
    );
    assert!(output.stderr.is_empty());
}
