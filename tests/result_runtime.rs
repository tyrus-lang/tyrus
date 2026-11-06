//! Native end-to-end coverage for Result success, error, propagation, and match.

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

struct ResultProject {
    root: PathBuf,
    name: String,
}

impl ResultProject {
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
        let name = format!("result-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Result integration test" }},
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

    fn emitted_ir(&self) -> String {
        fs::read_to_string(
            self.root
                .join("target/debug")
                .join(format!("{}.ll", self.name)),
        )
        .unwrap()
    }
}

impl Drop for ResultProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn result_success_error_propagation_and_match_run_natively() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): Result<i32, i32> {
    if (ok) { return Ok(42); }
    return Err(7);
}

function propagate(ok: boolean): Result<i32, i32> {
    const value = load(ok)?;
    return Ok(value);
}

function main(): void {
    const success = propagate(true);
    match (success) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
    const failure = propagate(false);
    match (failure) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\n7\n");
}

/// `?` used to become a `Try` only before a fixed list of tokens, so any
/// operator after it sent the parser into a ternary and failed with `Expected
/// Colon`. The parser now reads the would-be ternary branch speculatively.
#[test]
fn question_mark_is_followed_by_a_binary_operator() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): Result<i32, string> {
    if (ok) { return Ok(40); }
    return Err("boom");
}

function plus(ok: boolean): Result<i32, string> { return Ok(load(ok)? + 1); }
function minus(ok: boolean): Result<i32, string> { return Ok(load(ok)? - 1); }
function times(ok: boolean): Result<i32, string> { return Ok(load(ok)? * 2); }
function same(ok: boolean): Result<boolean, string> { return Ok(load(ok)? === 40); }

function main(): void {
    match (plus(true)) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
    match (minus(true)) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
    match (times(true)) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
    match (same(true)) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
    match (plus(false)) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
    const c = true;
    console.log(c ? -1 : 2);
    console.log(c ? false ? 7 : 8 : 9);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "41\n39\n80\ntrue\nboom\n-1\n8\n"
    );
}

/// A `?` inside a ternary arm produced invalid LLVM IR: the `Result` it
/// unwrapped was left pending and released after the merge, where it does not
/// dominate. It is now carried past the merge under an owned bit, so a heap
/// payload is neither leaked nor freed before the arm's value is used.
#[test]
fn question_mark_inside_a_ternary_arm() {
    let project = ResultProject::new(
        r#"
function name(ok: boolean): Result<string, string> {
    if (ok) { return Ok(`user-${40 + 2}`); }
    return Err("boom");
}

function inThen(ok: boolean, pick: boolean): Result<string, string> {
    return Ok(pick ? name(ok)? : "fallback");
}

function inElse(ok: boolean, pick: boolean): Result<string, string> {
    return Ok(pick ? "fallback" : name(ok)?);
}

function stolen(ok: boolean, wanted: boolean): Result<i32, string> {
    const next = number(ok)? + 1;
    return Ok(wanted ? number(ok)? - next : next);
}

function number(ok: boolean): Result<i32, string> {
    if (ok) { return Ok(42); }
    return Err("boom");
}

function showNumber(result: Result<i32, string>): void {
    match (result) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
}

function show(result: Result<string, string>): void {
    match (result) { Ok(v) => { console.log(v); } Err(e) => { console.log(e); } }
}

function main(): void {
    show(inThen(true, true));
    show(inThen(true, false));
    show(inThen(false, true));
    show(inElse(true, false));
    show(inElse(false, true));
    showNumber(stolen(true, true));
    showNumber(stolen(true, false));
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(
        project.emitted_ir().contains("ternary.result.owned.path"),
        "a ternary with one owned arm must carry a runtime ownership bit"
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "user-42\nfallback\nboom\nuser-42\nfallback\n-1\n43\n"
    );
}

#[test]
fn question_mark_propagates_a_result_with_an_allocated_error() {
    let project = ResultProject::new(
        r#"
function loadNumber(ok: boolean): Result<i32, string> {
    if (ok) { return Ok(42); }
    return Err("failed");
}

function propagate(ok: boolean): Result<i32, string> {
    return Ok(loadNumber(ok)?);
}

function main(): void {
    match (propagate(true)) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
    match (propagate(false)) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "42\nfailed\n");
}

#[test]
fn onfail_handles_a_result_error_then_propagates_it_implicitly() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): Result<i32, string> {
    console.log("load");
    if (ok) { return Ok(41); }
    return Err("failed".toUpperCase());
}

function work(ok: boolean): Result<i32, string> {
    defer console.log("defer");
    errdefer console.log("errdefer");
    const result = load(ok) onfail {
        console.log(result);
    };
    console.log(result);
    return Ok(result + 1);
}

function main(): void {
    match (work(true)) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
    match (work(false)) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.log(error); }
    }
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "load\n41\ndefer\n42\nload\nFAILED\nerrdefer\ndefer\nFAILED\n"
    );
}

#[test]
fn onfail_returns_from_a_void_function_after_a_result_error() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): Result<i32, string> {
    console.log("load");
    if (ok) { return Ok(41); }
    return Err("failed".toUpperCase());
}

function work(ok: boolean): void {
    const value = load(ok) onfail {
        console.log(value);
    };
    console.log("after onfail", value + 1);
}

function main(): void {
    work(true);
    work(false);
    console.log("after work");
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "load\nafter onfail 42\nload\nFAILED\nafter work\n"
    );
}

#[test]
fn onfail_handles_a_synchronous_throwing_call_in_a_void_function() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): i32 throws string {
    console.log("load");
    if (ok) { return 41; }
    throw "failed".toUpperCase();
}

function work(ok: boolean): void {
    const value = load(ok) onfail {
        console.log(value);
    };
    console.log(value + 1);
}

function main(): void {
    work(true);
    work(false);
    console.log("after");
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "load\n42\nload\nFAILED\nafter\n"
    );
}

#[test]
fn onfail_rethrows_to_the_enclosing_exception_boundary() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): i32 throws string {
    if (ok) { return 7; }
    throw "failed".toUpperCase();
}

function work(ok: boolean): i32 throws string {
    const value = load(ok) onfail console.log(value);
    return value;
}

function main(): void {
    try {
        console.log(work(true));
        console.log(work(false));
    } catch (error: string) {
        console.log(error);
    }
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "7\nFAILED\nFAILED\n");
}

#[test]
fn question_mark_propagates_inside_async_functions() {
    let project = ResultProject::new(
        r#"const parse = (ok: boolean): Result<i32, string> => {
    if (ok) {
        return Ok(40);
    }
    return Err(`bad ${"input"}`);
};

const load = async (ok: boolean): Promise<Result<i32, string>> => {
    if (ok) {
        return Ok(1);
    }
    return Err(`load ${"failed"}`);
};

const direct = async (ok: boolean): Promise<Result<i32, string>> => {
    const value = parse(ok)?;
    return Ok(value + 1);
};

const viaLocal = async (ok: boolean): Promise<Result<i32, string>> => {
    const result = await load(ok);
    const value = result?;
    return Ok(value + 100);
};

const retyped = async (ok: boolean): Promise<Result<string, string>> => {
    const value = parse(ok)?;
    await load(true);
    return Ok(`value ${value}`);
};

const withFinally = async (ok: boolean): Promise<Result<i32, string>> => {
    try {
        const value = parse(ok)?;
        return Ok(value);
    } catch (error) {
        printError(error.message);
        return Err("caught");
    } finally {
        await load(true);
        print("finally ran");
    }
};

const show = (label: string, result: Result<i32, string>): void => {
    match (result) {
        Ok(value) => { print(label, value); }
        Err(error) => { print(label, error); }
    }
};

const showText = (label: string, result: Result<string, string>): void => {
    match (result) {
        Ok(value) => { print(label, value); }
        Err(error) => { print(label, error); }
    }
};
const main = async (): Promise<void> => {
    show("direct", await direct(true));
    show("direct", await direct(false));
    show("local", await viaLocal(true));
    show("local", await viaLocal(false));
    showText("retyped", await retyped(true));
    showText("retyped", await retyped(false));
    show("finally", await withFinally(true));
    show("finally", await withFinally(false));
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "direct 41\ndirect bad input\nlocal 101\nlocal load failed\nretyped value 40\nretyped bad input\nfinally ran\nfinally 40\nfinally ran\nfinally bad input\n"
    );
}

#[test]
fn question_mark_over_a_local_propagates_a_copy_of_its_error() {
    let project = ResultProject::new(
        r#"
const parse = (ok: boolean): Result<i32, string> => {
    if (ok) {
        return Ok(40);
    }
    return Err(`bad ${"input"}`);
};

const widen = (ok: boolean): Result<i32[], string> => {
    const result = parse(ok);
    const value = result?;
    return Ok([value, value, value]);
};

const main = (): void => {
    match (widen(true)) {
        Ok(values) => { print(values.length, values[0]); }
        Err(error) => { print(error); }
    }
    match (widen(false)) {
        Ok(values) => { print(values.length); }
        Err(error) => { print(error); }
    }
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "3 40\nbad input\n");
}

#[test]
fn await_then_question_mark_and_hoisted_awaits_run_natively() {
    let project = ResultProject::new(
        r#"interface User {
    name: string;
}

const load = async (ok: boolean): Promise<Result<i32, string>> => {
    if (ok) {
        return Ok(40);
    }
    return Err(`load ${"failed"}`);
};

const user = async (ok: boolean): Promise<Result<User, string>> => {
    if (ok) {
        const found: User = { name: `Ada ${"L"}` };
        return Ok(found);
    }
    return Err("no user");
};

const next = async (ok: boolean): Promise<Result<i32, string>> => {
    const value = await load(ok)?;
    return Ok(value + 1);
};

const inline = async (ok: boolean): Promise<Result<i32, string>> => {
    return Ok(await load(ok)? + 2);
};

const field = async (ok: boolean): Promise<Result<string, string>> => {
    const name = (await user(ok)?).name;
    return Ok(name);
};

const main = async (): Promise<void> => {
    for (let round = 0; round < 2; round += 1) {
        match (await next(round === 0)) {
            Ok(value) => { print("next", value); }
            Err(error) => { print("next", error); }
        }
        match (await inline(round === 0)) {
            Ok(value) => { print("inline", value); }
            Err(error) => { print("inline", error); }
        }
        match (await field(round === 0)) {
            Ok(value) => { print("field", value); }
            Err(error) => { print("field", error); }
        }
    }
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(
        project
            .emitted_ir()
            .contains("zeroinitializer, ptr %\"async.frame.local.await#"),
        "`?` must disarm the consumed awaited Result on propagation"
    );
    assert!(
        project.emitted_ir().contains("async.return.source.result"),
        "async Result returns must release the source payload after cloning it into the Promise"
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "next 41\ninline 42\nfield Ada L\nnext load failed\ninline load failed\nfield no user\n"
    );
}

#[test]
fn void_results_and_borrowed_arrays_run_inside_async_functions() {
    let project = ResultProject::new(
        r#"// `Promise<Result<void, E>>` carries no success payload to clone or drop, and an
// async local that reads an array of objects out of other storage (a local, the
// success of `?` on one, a field) keeps a clone the frame then releases.
import { sleepAsync } from "tyrus:task";

interface Paint {
    name: string;
    shade: i32;
}

interface Palette {
    paints: Paint[];
}

interface Failure {
    code: string;
}

const load = async (ok: boolean): Promise<Result<Paint[], Failure>> => {
    if (ok) {
        const paints: Paint[] = [{ name: `red ${1}`, shade: 1 }, { name: "blue", shade: 2 }];
        return Ok(paints);
    }
    const failure: Failure = { code: `E${2}` };
    return Err(failure);
};

const check = async (ok: boolean): Promise<Result<void, Failure>> => {
    const paints = await load(ok)?;
    const palette: Palette = { paints: paints };
    const copy = palette.paints;
    await sleepAsync(1);
    print(copy.length, copy[0].name);
    return Ok();
};

const main = async (): Promise<void> => {
    for (let round = 0; round < 20; round += 1) {
        match (await check(round % 2 === 0)) {
            Ok() => {}
            Err(failure) => { print(failure.code); }
        }
    }
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "2 red 1\nE2\n".repeat(10)
    );
}

#[test]
fn onfail_on_a_result_leaves_a_void_function_when_its_handler_ends() {
    let project = ResultProject::new(
        r#"
function load(ok: boolean): Result<string, string> {
    if (ok) { return Ok(`value ${1}`); }
    return Err(`failed ${2}`);
}

function show(ok: boolean): void {
    const value = load(ok) onfail console.log("handled", value);
    console.log("loaded", value);
}

function main(): void {
    show(true);
    show(false);
    console.log("after");
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "loaded value 1\nhandled failed 2\nafter\n"
    );
}

#[test]
fn onfail_separates_the_err_of_an_awaited_result() {
    let project = ResultProject::new(
        r#"
import { sleepAsync } from "tyrus:task";

interface Failure {
    code: string;
}

const load = async (ok: boolean): Promise<Result<string[], Failure>> => {
    await sleepAsync(1);
    if (ok) {
        const names: string[] = [`red ${1}`, "blue"];
        return Ok(names);
    }
    const failure: Failure = { code: `E${2}` };
    return Err(failure);
};

const propagate = async (ok: boolean): Promise<Result<usize, Failure>> => {
    const names = await load(ok) onfail {
        print("propagating", names.code);
    };
    await sleepAsync(1);
    return Ok(names.length);
};

const explicit = async (ok: boolean): Promise<Result<string, Failure>> => {
    const names: string[] = await load(ok) onfail {
        return Err(names);
    };
    await sleepAsync(1);
    return Ok(names[0]);
};

const quiet = async (ok: boolean): Promise<void> => {
    const names = await load(ok) onfail print("quiet", names.code);
    await sleepAsync(1);
    print("quiet ok", names[0]);
};

const looped = async (): Promise<void> => {
    for (let round = 0; round < 4; round += 1) {
        const names = await load(round % 2 === 0) onfail {
            print("loop failed", names.code);
            continue;
        };
        print("loop", names.length);
    }
};

const main = async (): Promise<void> => {
    for (let round = 0; round < 6; round += 1) {
        const ok = round % 2 === 0;
        match (await propagate(ok)) {
            Ok(count) => { print("count", count); }
            Err(failure) => { print("err", failure.code); }
        }
        match (await explicit(ok)) {
            Ok(first) => { print("first", first); }
            Err(failure) => { print("err", failure.code); }
        }
        await quiet(ok);
    }
    await looped();
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "count 2\nfirst red 1\nquiet ok red 1\npropagating E2\nerr E2\nerr E2\nquiet E2\ncount 2\nfirst red 1\nquiet ok red 1\npropagating E2\nerr E2\nerr E2\nquiet E2\ncount 2\nfirst red 1\nquiet ok red 1\npropagating E2\nerr E2\nerr E2\nquiet E2\nloop 2\nloop failed E2\nloop 2\nloop failed E2\n"
    );
}

#[test]
fn a_compact_match_arm_returns_from_an_async_void_function() {
    let project = ResultProject::new(
        r#"
const describe = async (n: i32): Promise<void> => {
    match (n) {
        1 => console.log("one");
        _ => console.log("other");
    }
    console.log("unreachable");
};

const main = async (): Promise<void> => {
    await describe(1);
    await describe(2);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "one\nother\n");
}
