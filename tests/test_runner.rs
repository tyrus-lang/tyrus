//! End-to-end coverage for `tyrus test` and the `tyrus:test` runner: each case
//! writes a scratch project, runs the real CLI on it and checks the report,
//! the exit status and, where it matters, the order things ran in.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn project(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tyrus-{name}-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r##"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
        "description": "tyrus test runner test"
  }},
  "imports": {{
    "#": "./src"
  }},
  "dependencies": {{}}
}}
"##
        ),
    )
    .unwrap();
    root
}

fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

fn tyrus_test(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("test")
        .args(args)
        .current_dir(root)
        // The report must be byte-comparable whatever terminal runs the suite.
        .env("TYRUS_TEST_COLOR", "never")
        .output()
        .unwrap()
}

/// The program's stdout without the build's own progress lines, which are not
/// part of the test report.
fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("  Compiling ") && !line.starts_with("  Finished "))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// Remove variable per-test and summary durations before comparing the report.
fn report(output: &Output) -> String {
    stdout(output)
        .lines()
        .map(|line| {
            let line = match line.rfind(" (") {
                Some(index) if line.ends_with("s)") => &line[..index],
                _ => line,
            };
            let Some((text, duration)) = line.rsplit_once(' ') else {
                return line;
            };
            let is_passed_test = text.trim_start().starts_with("✓ ");
            let elapsed = duration
                .strip_suffix("ms")
                .or_else(|| duration.strip_suffix('s'));
            if is_passed_test && elapsed.is_some_and(|value| value.parse::<f64>().is_ok()) {
                text
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn describe_failure(output: &Output) -> String {
    format!(
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Hooks and tests run in Jest's order: `beforeAll`, then the suite's entries
/// in the order they were declared (a nested suite runs where it was written,
/// not before its parent's tests), then `afterAll`. Root-level hooks apply to
/// the whole file, and `beforeEach`/`afterEach` wrap every descendant test.
#[test]
fn runs_hooks_and_tests_in_declaration_order() {
    let root = project("test-runner-order");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from "tyrus:test";

beforeAll(() => { console.log("root beforeAll"); });
afterAll(() => { console.log("root afterAll"); });

describe("outer", () => {
    beforeEach(() => { console.log("outer beforeEach"); });
    afterEach(() => { console.log("outer afterEach"); });

    it("first", () => { console.log("first body"); });

    describe("inner", () => {
        it("nested", () => { console.log("nested body"); });
    });

    it("last", () => { console.log("last body"); });
});

it("top level", () => { expect(1).toBe(1); });

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    assert_eq!(
        report(&output),
        "Running 4 tests...\n\
         \n\
         root beforeAll\n\
         outer\n\
         outer beforeEach\n\
         first body\n\
         outer afterEach\n\
         \x20\x20✓ first\n\
         \x20\x20inner\n\
         outer beforeEach\n\
         nested body\n\
         outer afterEach\n\
         \x20\x20\x20\x20✓ nested\n\
         outer beforeEach\n\
         last body\n\
         outer afterEach\n\
         \x20\x20✓ last\n\
         ✓ top level\n\
         root afterAll\n\
         \n\
         4 passed, 0 failed"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A failed matcher reports the values that were compared, strings quoted, and
/// the run exits non-zero. Before, every failure read `test threw`: the wrapper
/// never marked the result slot, so the runtime discarded the message.
#[test]
fn failed_matchers_report_the_compared_values() {
    let root = project("test-runner-messages");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { it, expect } from "tyrus:test";

it("toBe", () => { expect(2 + 3).toBe(6); });
it("toEqual", () => { expect("abc").toEqual("abd"); });
it("negated", () => { expect(4).not.toBe(4); });
it("ordering", () => { expect(3).toBeGreaterThan(10); });
it("closeTo", () => { expect(1.5).toBeCloseTo(2); });
it("length", () => { expect("abc").toHaveLength(5); });
it("contains", () => { expect("hello").toContain("xyz"); });
it("truthy", () => { expect(false).toBeTruthy(); });

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = stdout(&output);
    for expected in [
        "expected 5 to be 6",
        "expected \"abc\" to equal \"abd\"",
        "expected 4 not to be 4",
        "expected 3 to be greater than 10",
        "expected 1.5 to be close to 2.0",
        "expected \"abc\" to have length 5",
        "expected \"hello\" to contain \"xyz\"",
        "expected false to be truthy",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(!text.contains("test threw"), "{text}");
    assert!(report(&output).ends_with("0 passed, 8 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// Built-in assertions in one runnable test report every failure while keeping
/// the test count honest: the callback is one failed test, not two. Each
/// detail starts with a terminal-linkable source location.
#[test]
fn built_in_assertions_accumulate_failures_with_source_locations() {
    let root = project("test-runner-assertion-collection");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { it } from "tyrus:test";

function double(value: i32): i32 { return value * 2; }

it("doubles", () => {
    assertEqual(double(21), 41);
    assert(double(0) === 1, "doubling zero stays zero");
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = stdout(&output);
    assert!(text.contains("✗ doubles (2 assertion failures)"), "{text}");
    assert!(
        text.contains("assertEqual failed: values are not equal"),
        "{text}"
    );
    assert!(text.contains("doubling zero stays zero"), "{text}");
    assert!(text.contains("__tests__/suite.test.tyr:6:5"), "{text}");
    assert!(text.contains("__tests__/suite.test.tyr:7:5"), "{text}");
    assert!(report(&output).ends_with("0 passed, 1 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// The value under test is evaluated exactly once, on the passing path and on
/// the failing one: the message reads the value the comparison used rather
/// than evaluating the expression again. An assertion inside a hot loop must
/// not grow the stack either, which a per-iteration slot would.
#[test]
fn assertions_evaluate_the_actual_once_and_run_in_loops() {
    let root = project("test-runner-single-eval");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { it, expect } from "tyrus:test";

let calls: i32 = 0;

function bump(): i32 {
    calls += 1;
    return calls;
}

function callCount(): i32 {
    return calls;
}

it("fails once", () => { expect(bump()).toBe(99); });
it("counted one call", () => { expect(callCount()).toBe(1); });
it("passes once", () => { expect(bump()).toBe(2); });
it("counted two calls", () => { expect(callCount()).toBe(2); });
it("loops", () => {
    for (let i: i32 = 0; i < 1000000; i += 1) {
        expect(i).toBeLessThan(1000000);
    }
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    let text = stdout(&output);
    assert!(
        text.contains("expected 1 to be 99"),
        "{}",
        describe_failure(&output)
    );
    assert!(report(&output).ends_with("4 passed, 1 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// A `describe` body whose assertion fails at registration time fails that
/// suite, attributed to that suite, even when it registered a nested suite
/// first. The id used to be re-derived after the callback, which named the
/// nested suite instead. (A bare `throw` there is a compile error, so a failed
/// `expect` is how a suite body fails at run time.)
#[test]
fn a_throwing_describe_fails_its_own_suite() {
    let root = project("test-runner-describe-throws");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

describe("broken", () => {
    describe("child", () => {
        it("never runs", () => { expect(1).toBe(1); });
    });
    it("also never runs", () => { expect(1).toBe(1); });
    expect(1).toBe(2);
});

describe("healthy", () => {
    it("still runs", () => { expect(1).toBe(1); });
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    assert!(
        text.contains("✗ broken (describe failed)\n    expected 1 to be 2"),
        "{text}"
    );
    assert!(!text.contains("child"), "{text}");
    assert!(text.contains("✓ still runs"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// The test framework never reaches a program: importing it outside a test
/// file is `E0434`, and a test file, which may import it, is still refused as
/// a program by `tyrus run`, since its runner executes when the process exits.
#[test]
fn production_builds_refuse_the_test_framework() {
    let root = project("test-runner-production");
    write(
        &root,
        "src/main.tyr",
        r##"import { it, expect } from "tyrus:test";

it("would run at exit", () => { expect(1).toBe(1); });

const main = (): void => {};
"##,
    );

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(build.status.code(), Some(1), "{}", describe_failure(&build));
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(stderr.contains("E0434"), "{stderr}");
    assert!(
        stderr.contains(
            "`tyrus:test` can only be imported by a test file, and `main.tyr` is not one"
        ),
        "{stderr}"
    );

    // A test file may import it, and is still not a program.
    write(&root, "src/main.tyr", "const main = (): void => {};\n");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r##"import { it, expect } from "tyrus:test";

it("runs under tyrus test", () => { expect(1).toBe(1); });
"##,
    );
    let ran = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["run", "__tests__/suite.test.tyr"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(ran.status.code(), Some(1), "{}", describe_failure(&ran));
    let stderr = String::from_utf8_lossy(&ran.stderr);
    assert!(
        stderr.contains("uses `tyrus:test`, which only `tyrus test` can run"),
        "{stderr}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A test file is found by its name anywhere in the project, `__tests__/`
/// being only the convention. A directory path narrows the run to the test
/// files under it, a path that is not a test file is refused, and a project
/// without test files says so.
#[test]
fn finds_test_files_by_name_and_narrows_by_path() {
    let root = project("test-runner-naming");
    write(&root, "src/main.tyr", "const main = (): void => {};\n");
    write(
        &root,
        "__tests__/a.test.tyr",
        "import { it, expect } from \"tyrus:test\";\n\nit(\"in tests folder\", () => { expect(1).toBe(1); });\n",
    );
    write(
        &root,
        "lib/b.test.tyr",
        "import { it, expect } from \"tyrus:test\";\n\nit(\"next to the code\", () => { expect(2).toBe(2); });\n",
    );

    let all = tyrus_test(&root, &[]);
    assert!(all.status.success(), "{}", describe_failure(&all));
    let text = report(&all);
    assert!(text.contains("__tests__/a.test.tyr"), "{text}");
    assert!(text.contains("lib/b.test.tyr"), "{text}");
    assert!(text.contains("Test files: 2 passed, 0 failed"), "{text}");

    let folder = tyrus_test(&root, &["__tests__"]);
    assert!(folder.status.success(), "{}", describe_failure(&folder));
    let text = report(&folder);
    assert!(text.contains("✓ in tests folder"), "{text}");
    assert!(!text.contains("next to the code"), "{text}");

    let not_a_test = tyrus_test(&root, &["src/main.tyr"]);
    assert_eq!(not_a_test.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&not_a_test.stderr);
    assert!(
        stderr.contains("`src/main.tyr` is not a test file"),
        "{stderr}"
    );
    assert!(stderr.contains("`__tests__/main.test.tyr`"), "{stderr}");
    fs::remove_dir_all(root).unwrap();

    let empty = project("test-runner-empty");
    write(&empty, "src/main.tyr", "const main = (): void => {};\n");
    let output = tyrus_test(&empty, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No tests found."), "{stderr}");
    fs::remove_dir_all(empty).unwrap();
}

/// A test may move from a flat path to a nested folder without an executable
/// from its previous artifact path blocking creation of the new directory.
#[test]
fn nested_test_artifact_path_does_not_collide_with_a_flat_test_artifact() {
    let root = project("test-runner-nested-artifact");
    fs::create_dir_all(root.join("target/test/__tests__")).unwrap();
    fs::write(
        root.join("target/test/__tests__/assertions"),
        "stale assertions.test.tyr executable",
    )
    .unwrap();
    write(
        &root,
        "__tests__/assertions/assertions.test.tyr",
        "import { it, expect } from \"tyrus:test\";\n\nit(\"runs from a nested folder\", () => { expect(1).toBe(1); });\n",
    );

    let output = tyrus_test(&root, &["__tests__/assertions/assertions.test.tyr"]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    assert!(
        root.join("target/test/files/__tests__/assertions/assertions")
            .is_file(),
        "nested test artifact was not created"
    );
    fs::remove_dir_all(root).unwrap();
}

/// `*.test.tyr` files are discovered wherever they are and each runs as its
/// own binary; the run ends with a combined summary. `src/main.tyr` is not a
/// test file and is never run by `tyrus test`.
#[test]
fn discovers_test_files_and_combines_their_results() {
    let root = project("test-runner-discovery");
    write(&root, "src/main.tyr", "const main = (): void => {};\n");
    write(
        &root,
        "src/math.test.tyr",
        r#"import { it, expect } from "tyrus:test";

it("adds", () => { expect(1 + 1).toBe(2); });
"#,
    );
    write(
        &root,
        "tests/strings/text.test.tyr",
        r#"import { it, expect } from "tyrus:test";

it("contains", () => { expect("hello").toContain("ell"); });
it("fails", () => { expect("a").toBe("b"); });
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    let math = text.find("src/math.test.tyr").expect(&text);
    let strings = text.find("tests/strings/text.test.tyr").expect(&text);
    assert!(math < strings, "{text}");
    assert!(
        text.contains("----------------------------------------\nTest files:"),
        "{text}"
    );
    assert!(text.contains("Test files: 1 passed, 1 failed"), "{text}");
    assert!(text.ends_with("Tests: 2 passed, 1 failed"), "{text}");

    // A single path runs only that file, with the runner's own report.
    let single = tyrus_test(&root, &["src/math.test.tyr"]);
    assert!(single.status.success(), "{}", describe_failure(&single));
    assert!(
        !report(&single).contains("Test files:"),
        "{}",
        report(&single)
    );
    fs::remove_dir_all(root).unwrap();
}

/// Outside a project, `tyrus test` names the missing file rather than falling
/// through to "no tests found"; a bad flag fails before anything compiles.
#[test]
fn reports_a_missing_project_and_bad_flags() {
    let root = std::env::temp_dir().join(format!(
        "tyrus-test-runner-no-project-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let output = tyrus_test(&root, &[]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`tyrus.json` not found"), "{stderr}");
    fs::remove_dir_all(&root).unwrap();

    let project = project("test-runner-bad-flag");
    let output = tyrus_test(&project, &["--wat"]);
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown option `--wat`"), "{stderr}");
    fs::remove_dir_all(project).unwrap();
}

/// `.skip` keeps a test or a suite listed without running it (a skipped
/// suite's hooks do not run either), and `test.todo` lists a placeholder.
/// Both are counted in the summary.
#[test]
fn skip_and_todo_are_listed_without_running() {
    let root = project("test-runner-skip-todo");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, test, it, expect, beforeAll } from "tyrus:test";

describe("modifiers", () => {
    test.skip("skipped", () => { expect(1).toBe(2); });
    test.todo("write this later");
    it("runs", () => { expect(1).toBe(1); });
});

describe.skip("skipped suite", () => {
    beforeAll(() => { console.log("skipped suite setup"); });
    it("inner", () => { expect(1).toBe(2); });
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    assert_eq!(
        report(&output),
        "Running 1 test...\n\n\
         modifiers\n\
         \x20\x20○ skipped (skipped)\n\
         \x20\x20○ write this later (todo)\n\
         \x20\x20✓ runs\n\
         skipped suite\n\
         \x20\x20○ inner (skipped)\n\
         \n\
         1 passed, 0 failed, 2 skipped, 1 todo"
    );
    fs::remove_dir_all(root).unwrap();
}

/// With any test or suite focused by `.only`, only focused tests run and the
/// rest count as skipped, while `.skip` still wins over `.only`.
#[test]
fn only_focuses_the_file() {
    let root = project("test-runner-only");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

it("not focused", () => { expect(1).toBe(2); });
it.only("focused", () => { expect(1).toBe(1); });
describe.only("focused suite", () => {
    it("inside", () => { expect(2).toBe(2); });
    it.skip("skip wins", () => { expect(1).toBe(2); });
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    assert_eq!(
        report(&output),
        "Running 2 tests...\n\n\
         ✓ focused\n\
         focused suite\n\
         \x20\x20✓ inside\n\
         \x20\x20○ skip wins (skipped)\n\
         \n\
         2 passed, 0 failed, 2 skipped"
    );
    fs::remove_dir_all(root).unwrap();
}

/// `toThrow` calls the function through the language's own `try`/`catch` and
/// reports each way it can fail with the function's name and, for a thrown
/// `string`, what was actually thrown.
#[test]
fn to_throw_reports_every_failure_shape() {
    let root = project("test-runner-to-throw");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { it, expect } from "tyrus:test";

const boom = (): void throws string => {
    throw "boom";
};

const calm = (): void throws string => {
    if (false) {
        throw "never";
    }
};

it("throws", () => { expect(boom).toThrow(); });
it("throws the message", () => { expect(boom).toThrow("boom"); });
it("keeps calm", () => { expect(calm).not.toThrow(); });
it("avoids one message", () => { expect(boom).not.toThrow("other"); });
it("wrong message", () => { expect(boom).toThrow("bam"); });
it("unexpected throw", () => { expect(boom).not.toThrow(); });
it("missing throw", () => { expect(calm).toThrow(); });
it("forbidden message", () => { expect(boom).not.toThrow("boom"); });

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    for expected in [
        "✗ wrong message\n  expected boom to throw \"bam\", but it threw \"boom\"",
        "✗ unexpected throw\n  expected boom not to throw, but it threw \"boom\"",
        "✗ missing throw\n  expected calm to throw",
        "✗ forbidden message\n  expected boom not to throw \"boom\"",
    ] {
        assert!(text.contains(expected), "missing `{expected}` in:\n{text}");
    }
    assert!(text.ends_with("4 passed, 4 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// Modifiers are resolved through the import, so they work on aliased
/// registrars, and a file whose only tests are modifiers still counts as a
/// test file.
#[test]
fn modifiers_work_through_import_aliases() {
    let root = project("test-runner-alias");
    write(&root, "src/main.tyr", "const main = (): void => {};\n");
    write(
        &root,
        "tests/alias.test.tyr",
        r#"import { describe as suite, test as check, expect } from "tyrus:test";

suite.only("focused through an alias", () => {
    check("runs", () => { expect(1).toBe(1); });
});
check.skip("skipped through an alias", () => { expect(1).toBe(2); });
"#,
    );
    write(
        &root,
        "tests/todo.test.tyr",
        "import { test } from \"tyrus:test\";\n\ntest.todo(\"nothing written yet\");\n",
    );

    let output = tyrus_test(&root, &[]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    let text = report(&output);
    assert!(
        text.contains("focused through an alias\n  ✓ runs"),
        "{text}"
    );
    assert!(
        text.contains("○ skipped through an alias (skipped)"),
        "{text}"
    );
    assert!(text.contains("○ nothing written yet (todo)"), "{text}");
    assert!(text.contains("Test files: 2 passed, 0 failed"), "{text}");
    assert!(
        text.ends_with("Tests: 1 passed, 0 failed, 1 skipped, 1 todo"),
        "{text}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// `async` tests and hooks are awaited before the next step runs, including
/// `const value = await f();` inside an `async` callback nested in a
/// `describe`, and a rejected expectation fails the test with its message.
#[test]
fn async_tests_and_hooks_are_awaited() {
    let root = project("test-runner-async");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect, beforeEach } from "tyrus:test";
import { sleepAsync } from "tyrus:task";

async function fetchCount(): Promise<i32> {
    await sleepAsync(2);
    return 3;
}

describe("async", () => {
    beforeEach(async () => {
        await sleepAsync(1);
        console.log("async hook ran");
    });

    it("awaits a value", async () => {
        const count = await fetchCount();
        expect(count).toBe(3);
    });

    it("fails after awaiting", async () => {
        const count = await fetchCount();
        expect(count).toBe(4);
    });
});

const main = (): void => {};
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    assert_eq!(text.matches("async hook ran").count(), 2, "{text}");
    assert!(text.contains("✓ awaits a value"), "{text}");
    assert!(
        text.contains("✗ fails after awaiting\n    expected 3 to be 4"),
        "{text}"
    );
    assert!(text.ends_with("1 passed, 1 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// Only a test file may import `tyrus:test`. A helper module that reaches for
/// it is rejected even when a test file is what imports the helper, because
/// the module arms a runner that executes when the program exits.
#[test]
fn only_test_files_may_import_the_framework() {
    let root = project("test-runner-import-rule");
    write(
        &root,
        "src/helpers.tyr",
        r##"import { expect } from "tyrus:test";

export const expectPositive = (value: i32): void => {
    expect(value).toBeGreaterThan(0);
};
"##,
    );
    write(
        &root,
        "__tests__/uses-helper.test.tyr",
        r##"import { it } from "tyrus:test";
import { expectPositive } from "#/helpers";

it("delegates to the helper", () => { expectPositive(1); });
"##,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("E0434"), "{stderr}");
    assert!(
        stderr.contains(
            "`tyrus:test` can only be imported by a test file, and `helpers.tyr` is not one"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains("rename it to `helpers.test.tyr`"),
        "{stderr}"
    );

    // Renaming the helper is the fix the diagnostic points at. The renamed
    // file is itself a test file that registers no test, which contributes
    // nothing rather than failing the run.
    fs::rename(
        root.join("src/helpers.tyr"),
        root.join("src/helpers.test.tyr"),
    )
    .unwrap();
    write(
        &root,
        "__tests__/uses-helper.test.tyr",
        r##"import { it } from "tyrus:test";
import { expectPositive } from "#/helpers.test";

it("delegates to the helper", () => { expectPositive(1); });
"##,
    );
    let output = tyrus_test(&root, &[]);
    assert!(output.status.success(), "{}", describe_failure(&output));
    fs::remove_dir_all(root).unwrap();
}

/// A new project ships a passing test: `tyrus new` writes `__tests__/` with
/// one, so the first `tyrus test` reports a result instead of "No tests
/// found.", and `tyrus run` still builds the program itself.
#[test]
fn a_new_project_ships_a_passing_test() {
    let parent = std::env::temp_dir().join(format!(
        "tyrus-new-scaffold-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&parent).unwrap();
    let created = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["new", "scaffolded"])
        .current_dir(&parent)
        .output()
        .unwrap();
    assert!(created.status.success(), "{}", describe_failure(&created));

    let root = parent.join("scaffolded");
    assert!(root.join("__tests__/example.test.tyr").exists());
    assert!(!root.join("src/greeting.tyr").exists());

    let tested = tyrus_test(&root, &[]);
    assert!(tested.status.success(), "{}", describe_failure(&tested));
    let text = report(&tested);
    assert!(text.contains("✓ adds the prices"), "{text}");
    assert!(text.ends_with("1 passed, 0 failed"), "{text}");

    let ran = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("run")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(ran.status.success(), "{}", describe_failure(&ran));
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "Hello World!\n",
        "{}",
        describe_failure(&ran)
    );
    fs::remove_dir_all(parent).unwrap();
}

/// `toEqual` compares arrays element by element, with the element equality the
/// array methods already use: order and length matter, `-0.0` equals `0.0`,
/// and `NaN` equals nothing. Arrays of objects and arrays of arrays take the
/// structural path instead, covered by its own test.
#[test]
fn to_equal_compares_arrays_element_by_element() {
    let root = project("test-runner-array-equality");
    write(
        &root,
        "__tests__/arrays.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

describe("arrays", () => {
    it("equal numbers", () => { expect([1, 2, 3]).toEqual([1, 2, 3]); });
    it("equal strings", () => { expect(["ada", "grace"]).toEqual(["ada", "grace"]); });
    it("empty", () => {
        const empty: i32[] = [];
        expect(empty).toEqual([]);
    });
    it("negative zero equals zero", () => { expect([0.0]).toEqual([-0.0]); });
    it("order matters", () => { expect([1, 2]).not.toEqual([2, 1]); });
    it("length matters", () => { expect([1, 2]).not.toEqual([1, 2, 3]); });
    it("nan equals nothing", () => {
        const nan: f64[] = [0.0 / 0.0];
        expect(nan).not.toEqual(nan);
    });
    it("reports a difference", () => { expect([1, 2, 3]).toEqual([1, 9, 3]); });
});
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    assert!(
        text.contains("✗ reports a difference\n    expected the arrays to be equal"),
        "{text}"
    );
    assert!(text.ends_with("7 passed, 1 failed"), "{text}");

    fs::remove_dir_all(root).unwrap();
}

/// Test files run in parallel by default, one child process per file, and the
/// report is grouped per file in discovery order however the processes
/// finished.
#[test]
fn test_files_run_in_parallel_and_report_in_discovery_order() {
    let root = project("test-runner-parallel");
    for index in 1..=4 {
        write(
            &root,
            &format!("__tests__/f{index}.test.tyr"),
            &format!(
                "import {{ describe, it, expect }} from \"tyrus:test\";\n\ndescribe(\"file {index}\", () => {{\n    it(\"passes {index}\", () => {{ expect({index}).toBe({index}); }});\n}});\n"
            ),
        );
    }

    let parallel = tyrus_test(&root, &[]);
    assert!(parallel.status.success(), "{}", describe_failure(&parallel));
    let text = report(&parallel);
    let positions: Vec<usize> = (1..=4)
        .map(|index| {
            text.find(&format!("__tests__/f{index}.test.tyr"))
                .unwrap_or_else(|| panic!("file {index} missing in:\n{text}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "files out of discovery order:\n{text}"
    );
    assert!(text.contains("Test files: 4 passed, 0 failed"), "{text}");
    assert!(text.ends_with("Tests: 4 passed, 0 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// `toEqual` compares objects field by field, the comparison the automatic
/// `equals` performs, and `toContain` searches an array with the search
/// `includes` answers. Neither an object nor an array can be interpolated into
/// a message, so a failure names what was compared instead.
#[test]
fn matchers_compare_objects_and_search_arrays() {
    let root = project("test-runner-structural");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

interface Point {
    x: i32;
    y: i32;
}

describe("objects", () => {
    it("equal", () => {
        const left: Point = { x: 1, y: 2 };
        const right: Point = { y: 2, x: 1 };
        expect(left).toEqual(right);
    });
    it("differ", () => {
        const left: Point = { x: 1, y: 2 };
        const right: Point = { x: 1, y: 3 };
        expect(left).toEqual(right);
    });
    it("negated", () => {
        const left: Point = { x: 1, y: 2 };
        const right: Point = { x: 9, y: 9 };
        expect(left).not.toEqual(right);
    });
});

describe("contains", () => {
    it("numbers", () => { expect([1, 2, 3]).toContain(2); });
    it("strings", () => { expect(["ada", "grace"]).toContain("ada"); });
    it("substring", () => { expect("hello").toContain("ell"); });
    it("missing", () => { expect([1, 2, 3]).toContain(9); });
    it("negated", () => { expect([1, 2]).not.toContain(5); });
});
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    assert!(
        text.contains("✗ differ\n    expected the objects to be equal"),
        "{text}"
    );
    assert!(
        text.contains("✗ missing\n    expected the array to contain 9"),
        "{text}"
    );
    for passing in [
        "✓ equal",
        "✓ negated",
        "✓ numbers",
        "✓ strings",
        "✓ substring",
    ] {
        assert!(text.contains(passing), "missing `{passing}` in:\n{text}");
    }
    assert!(text.ends_with("6 passed, 2 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// `test.each` registers one test per case, named `name [index]`. The callback
/// may be written inline, with its parameter type inferred from the cases, or
/// named and called with each case. The cases are unrolled when the program is
/// compiled, so a list built at run time and a callback that takes more than
/// one case are both refused with a place to point at.
#[test]
fn test_each_registers_one_test_per_case() {
    let root = project("test-runner-each");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, test, it, expect } from "tyrus:test";

const double = (value: i32): i32 => {
    return value * 2;
};

const checkCase = (value: i32): void => {
    expect(double(value)).toBeGreaterThan(value);
};

describe("double", () => {
    test.each("doubles", [1, 2, 3], (value: i32) => {
        expect(double(value)).toBe(value * 2);
    });

    it.each("named callback", [4, 5], checkCase);

    test.each("infers the parameter", ["ada", "grace"], (name) => {
        expect(name).toHaveLength(3);
    });
});
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    for passing in [
        "✓ doubles [0]",
        "✓ doubles [1]",
        "✓ doubles [2]",
        "✓ named callback [0]",
        "✓ named callback [1]",
        "✓ infers the parameter [0]",
    ] {
        assert!(text.contains(passing), "missing `{passing}` in:\n{text}");
    }
    // The inferred parameter is a `string`, so the failure renders the case.
    assert!(
        text.contains("✗ infers the parameter [1]\n    expected \"grace\" to have length 3"),
        "{text}"
    );
    assert!(text.ends_with("6 passed, 1 failed"), "{text}");

    // Cases built at run time, and a callback of the wrong shape, are refused.
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { test, expect } from "tyrus:test";

const cases: i32[] = [1, 2];

test.each("runtime", cases, (value: i32) => { expect(value).toBe(value); });
test.each("two", [1], (left: i32, right: i32) => { expect(left).toBe(right); });
"#,
    );
    let refused = tyrus_test(&root, &[]);
    assert_eq!(
        refused.status.code(),
        Some(1),
        "{}",
        describe_failure(&refused)
    );
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("E0658"), "{stderr}");
    assert!(
        stderr.contains("`test.each` needs its cases written inline, as an array literal"),
        "{stderr}"
    );
    assert!(stderr.contains("E0057"), "{stderr}");
    assert!(
        stderr.contains("takes exactly one case, but it declares 2"),
        "{stderr}"
    );
    fs::remove_dir_all(root).unwrap();
}

/// Equality reaches all the way down: an array of objects compares its
/// elements field by field, and an array of arrays compares its inner arrays,
/// each stopping at the first difference.
#[test]
fn to_equal_compares_arrays_of_objects_and_nested_arrays() {
    let root = project("test-runner-deep-equality");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

interface Point {
    x: i32;
    y: i32;
}

describe("arrays of objects", () => {
    it("equal", () => {
        const left: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4 }];
        const right: Point[] = [{ y: 2, x: 1 }, { y: 4, x: 3 }];
        expect(left).toEqual(right);
    });
    it("a field differs", () => {
        const left: Point[] = [{ x: 1, y: 2 }];
        const right: Point[] = [{ x: 1, y: 9 }];
        expect(left).not.toEqual(right);
    });
    it("the length differs", () => {
        const left: Point[] = [{ x: 1, y: 2 }];
        const right: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4 }];
        expect(left).not.toEqual(right);
    });
});

describe("nested arrays", () => {
    it("equal", () => {
        const left: i32[][] = [[1, 2], [3]];
        const right: i32[][] = [[1, 2], [3]];
        expect(left).toEqual(right);
    });
    it("an inner element differs", () => {
        const left: i32[][] = [[1, 2], [3]];
        const right: i32[][] = [[1, 2], [4]];
        expect(left).not.toEqual(right);
    });
    it("empty inner arrays", () => {
        const left: i32[][] = [[]];
        const right: i32[][] = [[]];
        expect(left).toEqual(right);
    });
    it("reports a difference", () => {
        const left: i32[][] = [[1]];
        const right: i32[][] = [[2]];
        expect(left).toEqual(right);
    });
});
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    assert!(
        text.contains("✗ reports a difference\n    expected the arrays to be equal"),
        "{text}"
    );
    assert!(text.ends_with("6 passed, 1 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}

/// `.rejects` settles the promise and asserts that it failed, with an optional
/// message and the `.not` form. It settles through the runtime rather than
/// suspending, so it works in a synchronous test, and every failure names what
/// the promise actually did.
#[test]
fn rejects_asserts_a_failed_promise() {
    let root = project("test-runner-rejects");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { describe, it, expect } from "tyrus:test";

const boom = (): Promise<i32> => {
    return Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" });
};

const fine = (): Promise<i32> => {
    return Promise.resolve<i32>(3);
};

describe("rejects", () => {
    it("rejected", () => { expect(boom()).rejects.toThrow(); });
    it("rejected with the message", () => { expect(boom()).rejects.toThrow("boom"); });
    it("a resolved promise is not a rejection", () => { expect(fine()).not.rejects.toThrow(); });
    it("fails when it resolves", () => { expect(fine()).rejects.toThrow(); });
    it("fails on a different message", () => { expect(boom()).rejects.toThrow("other"); });
    it("fails when it rejected", () => { expect(boom()).not.rejects.toThrow(); });
});
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    for expected in [
        "✗ fails when it resolves\n    expected the promise to reject, but it resolved",
        "✗ fails on a different message\n    expected the promise to reject with \"other\", but it rejected with \"boom\"",
        "✗ fails when it rejected\n    expected the promise not to reject, but it rejected with \"boom\"",
    ] {
        assert!(text.contains(expected), "missing:\n{expected}\nin:\n{text}");
    }
    assert!(text.ends_with("3 passed, 3 failed"), "{text}");

    fs::remove_dir_all(root).unwrap();
}

/// The substring matchers must pass the position to start searching from.
///
/// `tyrus_string_index_of` takes it as a third argument, and the matcher used
/// to declare and call the function with two. The missing argument read
/// whatever the register held: normally zero, so every test here passed, and
/// under AddressSanitizer something else, so the search began past the end of
/// the string and `toContain` answered false for a substring plainly present.
///
/// A behavioural test cannot catch that, since the wrong code behaves
/// correctly by luck. The emitted IR can: the call has to carry three
/// arguments, and the declaration has to agree with the runtime's signature.
#[test]
fn substring_matchers_search_from_the_start_of_the_string() {
    let root = project("test-runner-substring-abi");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r#"import { it, expect } from "tyrus:test";

it("contains", () => { expect("hello").toContain("ell"); });
it("matches", () => { expect("hello").toMatch("llo"); });
"#,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        describe_failure(&output)
    );

    let ir = fs::read_to_string(root.join("target/test/files/__tests__/suite.ll"))
        .expect("the compiler writes the module next to the test binary");
    assert!(
        ir.contains("declare i64 @tyrus_string_index_of({ ptr, i64 }, { ptr, i64 }, i64)"),
        "the declaration must match the runtime's three-parameter signature:\n{ir}"
    );
    let calls: Vec<&str> = ir
        .lines()
        .filter(|line| line.contains("call i64 @tyrus_string_index_of("))
        .collect();
    assert_eq!(calls.len(), 2, "{calls:#?}");
    for call in calls {
        assert!(
            call.trim_end().ends_with("i64 0)"),
            "the search must start at zero, not at whatever the register held:\n{call}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

/// `.resolves` settles the promise and hands the matcher the value it resolved
/// to, so every matcher works after it and no `await` is needed. A promise that
/// rejected fails the assertion instead, naming what it rejected with.
#[test]
fn resolves_asserts_the_value_a_promise_resolved_to() {
    let root = project("test-runner-resolves");
    write(
        &root,
        "__tests__/suite.test.tyr",
        r##"import { describe, it, expect } from "tyrus:test";

interface Point {
    x: i32;
    y: i32;
}

const count = (): Promise<i32> => {
    return Promise.resolve<i32>(7);
};

const label = (): Promise<string> => {
    return Promise.resolve<string>("hello");
};

const corner = (): Promise<Point> => {
    return Promise.resolve<Point>({ x: 1, y: 2 });
};

const digits = (): Promise<i32[]> => {
    return Promise.resolve<i32[]>([1, 2, 3]);
};

const boom = (): Promise<i32> => {
    return Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" });
};

describe("resolves", () => {
    it("a number", () => { expect(count()).resolves.toBe(7); });
    it("a string", () => { expect(label()).resolves.toBe("hello"); });
    it("an object", () => { expect(corner()).resolves.toEqual({ x: 1, y: 2 }); });
    it("an array", () => { expect(digits()).resolves.toEqual([1, 2, 3]); });
    it("any matcher", () => { expect(count()).resolves.toBeGreaterThan(3); });
    it("negated", () => { expect(count()).resolves.not.toBe(8); });
    it("fails on the wrong value", () => { expect(count()).resolves.toBe(8); });
    it("fails when it rejects", () => { expect(boom()).resolves.toBe(7); });
});
"##,
    );

    let output = tyrus_test(&root, &[]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        describe_failure(&output)
    );
    let text = report(&output);
    for expected in [
        "✗ fails on the wrong value\n    expected 7 to be 8",
        "✗ fails when it rejects\n    expected the promise to resolve, but it rejected with \"boom\"",
    ] {
        assert!(text.contains(expected), "missing:\n{expected}\nin:\n{text}");
    }
    assert!(text.ends_with("6 passed, 2 failed"), "{text}");
    fs::remove_dir_all(root).unwrap();
}
