//! Native end-to-end coverage for async state-machine control flow.

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

fn normalize_windows_output(mut output: Output) -> Output {
    if cfg!(windows) {
        fn normalize(bytes: Vec<u8>) -> Vec<u8> {
            let mut result = Vec::with_capacity(bytes.len());
            let mut bytes = bytes.into_iter().peekable();
            while let Some(byte) = bytes.next() {
                if byte == b'\r' && bytes.peek() == Some(&b'\n') {
                    continue;
                }
                result.push(byte);
            }
            result
        }
        output.stdout = normalize(output.stdout);
        output.stderr = normalize(output.stderr);
    }
    output
}

fn process_status_description(status: &std::process::ExitStatus) -> String {
    #[cfg(windows)]
    {
        match status.code() {
            Some(code) => {
                let status_code = code as u32;
                let description = match status_code {
                    0xC000_0005 => "STATUS_ACCESS_VIOLATION",
                    0xC000_0374 => "STATUS_HEAP_CORRUPTION",
                    _ => "unknown Windows exception",
                };
                format!("{status:?} (Windows status 0x{status_code:08X}, {description})")
            }
            None => format!("{status:?} (Windows status unavailable)"),
        }
    }
    #[cfg(not(windows))]
    {
        format!("{status:?}")
    }
}

fn assert_no_unexpected_stderr(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if std::env::var_os("TYRUS_JIT_TRACE").is_some() {
        assert!(
            stderr.lines().all(|line| line.starts_with("JIT trace:")),
            "unexpected program stderr:\n{stderr}"
        );
    } else {
        assert!(stderr.is_empty(), "unexpected program stderr:\n{stderr}");
    }
}

fn loopback_bind_is_denied() -> bool {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            drop(listener);
            false
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: loopback socket tests require permission to bind a local TCP listener ({error})."
            );
            true
        }
        Err(error) => panic!("could not bind the loopback test listener: {error}"),
    }
}

struct AsyncProject {
    root: PathBuf,
    name: String,
}

impl AsyncProject {
    fn new(source: &str) -> Self {
        Self::with_runtime_config(source, "")
    }

    /// Same project, plus a raw `"runtime"` entry in `tyrus.json`.
    ///
    /// The section is written verbatim so a test can also cover a value the
    /// configuration rejects, which a typed parameter could not express.
    fn with_runtime_config(source: &str, runtime: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("async-runtime-{suffix}");
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
    "description": "Async runtime integration test"
  }},{runtime}
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
        let binary = self.root.join("target/debug").join(&self.name);
        normalize_windows_output(Command::new(binary).output().unwrap())
    }

    fn run_with_env(&self, vars: &[(&str, &str)]) -> Output {
        let binary = self.root.join("target/debug").join(&self.name);
        let mut command = Command::new(binary);
        for (name, value) in vars {
            command.env(name, value);
        }
        normalize_windows_output(command.output().unwrap())
    }

    /// The LLVM IR the build left behind, for asserting on what was emitted
    /// rather than only on what the program printed.
    fn emitted_ir(&self) -> String {
        fs::read_to_string(
            self.root
                .join("target/debug")
                .join(format!("{}.ll", self.name)),
        )
        .unwrap()
    }
}

/// The body of one generated function, so an assertion names where it expects
/// something rather than searching the whole module for it.
///
/// A release emitted in the wrong function is a different defect from a release
/// not emitted at all, and searching the module text cannot tell them apart.
fn generated_function<'ir>(ir: &'ir str, name: &str) -> &'ir str {
    // Matched on the definition line rather than on the name alone: every call
    // site spells the name too, and a match on one of those would hand back
    // whatever function happens to contain the call.
    let signature = format!(" @{name}(");
    let start = ir
        .match_indices("\ndefine ")
        .map(|(index, _)| index + 1)
        .find(|&start| {
            ir[start..]
                .lines()
                .next()
                .is_some_and(|line| line.contains(&signature))
        })
        .unwrap_or_else(|| panic!("`{name}` is missing from the emitted module:\n{ir}"));
    let body = &ir[start..];
    let end = body
        .find("\n}")
        .unwrap_or_else(|| panic!("`{name}` is never closed in the emitted module:\n{ir}"));
    &body[..end]
}

/// The drop glue an async function's frame runs when it is released.
fn async_drop_function<'ir>(ir: &'ir str, function: &str) -> &'ir str {
    generated_function(ir, &format!("__tyrus_async_drop_{function}"))
}

/// The state machine an async function's frame is resumed through.
fn async_poll_function<'ir>(ir: &'ir str, function: &str) -> &'ir str {
    generated_function(ir, &format!("__tyrus_async_poll_{function}"))
}

/// A worker-thread count this machine will certainly consider oversubscribed,
/// while staying small enough to actually start.
fn oversubscribed_worker_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |value| value.get()) * 2 + 1
}

#[test]
fn onfail_handles_an_await_rejection_before_an_outer_finally() {
    let project = AsyncProject::new(
        r#"async function main(): Promise<void> {
    try {
        const ready = await Promise.resolve<i32>(7) onfail {
            console.log(ready.message);
        };
        console.log(ready);
        const value = await Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" }) onfail {
            console.log(value.message);
        };
        console.log(value);
    } catch (error: Error) {
        console.log(error.message);
    } finally {
        console.log("finally");
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
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "7\nboom\nfinally\n"
    );
}

#[test]
fn onfail_rejects_a_non_void_async_function_after_handling_the_error() {
    let project = AsyncProject::new(
        r#"async function work(): Promise<i32> throws Error {
    const value = await Promise.reject<i32>({ message: "boom", code: "ERR_BOOM" }) onfail {
        console.log(value.message);
    };
    return value;
}

async function main(): Promise<void> {
    try {
        const value = await work();
        console.log(value);
    } catch (error: Error) {
        console.log(error.message);
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
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "boom\nboom\n");
}

impl Drop for AsyncProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn task_sleep_runs_synchronously_without_await() {
    let project = AsyncProject::new(
        r#"import { sleep } from "tyrus:task";

function main(): void {
    sleep(1);
    console.log("slept");
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "slept\n");
}

#[test]
#[allow(clippy::too_many_lines)]
fn switch_cases_suspend_resume_fall_through_and_bypass_unselected_awaits() {
    let project = AsyncProject::new(
        r#"async function selectedMode(mode: string): Promise<string> {
    console.log("subject");
    return mode;
}

async function choose(mode: string): Promise<void> {
    let trace: i32 = 0;
    switch (await selectedMode(mode)) {
        case "stop": {
            trace = 7;
            break;
        }
        default: {
            trace = 4;
            await Promise.resolve<void>();
        }
        case "await": {
            trace = trace * 10 + 1;
            const prefix: i32 = trace;
            const value: i32 = await Promise.resolve<i32>(2);
            const suffix: i32 = value;
            await Promise.resolve<void>();
            trace = prefix * 10 + suffix;
        }
        case "tail": {
            const offset: i32 = trace;
            const tail: i32 = await Promise.resolve<i32>(3);
            trace = offset * 10 + tail;
            break;
        }
    }
    console.log(trace);
}

async function rejectSelectedCase(): Promise<void> {
    switch ("reject") {
        case "reject": {
            await Promise.resolve<void>();
            await Promise.reject<void>({ message: "switch rejected", code: "ERR_SWITCH_REJECTED" });
            break;
        }
        default: {
            break;
        }
    }
}

async function rejectSubject(): Promise<void> {
    switch (await Promise.reject<string>({ message: "subject rejected", code: "ERR_SUBJECT_REJECTED" })) {
        default: {
            break;
        }
    }
}

async function subjectOnly(): Promise<void> {
    let trace: i32 = 0;
    switch (await Promise.resolve<i32>(2)) {
        case 2: {
            trace = 22;
            break;
        }
        default: {
            trace = -1;
        }
    }
    console.log(trace);
}

async function ownSelectedPayload(): Promise<void> {
    switch ("owned") {
        case "owned": {
            const label: string = await Promise.resolve<string>("owned switch payload");
            await Promise.resolve<void>();
            console.log(label);
            break;
        }
        default: {
            break;
        }
    }
}

async function awaitedCase(label: string): Promise<string> {
    console.log(label);
    return label;
}

async function selectAwaitedCase(subject: string): Promise<void> {
    switch (subject) {
        case "sync": {
            console.log("sync selected");
            break;
        }
        case await awaitedCase("first"): {
            console.log("first selected");
            break;
        }
        case await awaitedCase("second"): {
            console.log("second selected");
            break;
        }
        default: {
            console.log("case default");
        }
    }
}

async function selectExpressionCase(subject: i32): Promise<void> {
    switch (subject) {
        case (await Promise.resolve<i32>(1)) + (await Promise.resolve<i32>(2)): {
            console.log("expression selected");
            break;
        }
        case (await Promise.resolve<i32>(4)) + 1: {
            console.log("expression five");
            break;
        }
        default: {
            console.log("expression default");
        }
    }
}

async function selectOnlyMultiAwaitCase(subject: i32): Promise<void> {
    switch (subject) {
        case (await Promise.resolve<i32>(6)) + (await Promise.resolve<i32>(7)): {
            console.log("single multi-await selected");
            break;
        }
        default: {
            console.log("single multi-await default");
        }
    }
}

async function selectOwnedExpressionCase(subject: string): Promise<void> {
    switch (subject) {
        case (await Promise.resolve<string>("owned")).toUpperCase(): {
            console.log("owned expression selected");
            break;
        }
        default: {
            console.log("owned expression default");
        }
    }
}

async function tracedCase(): Promise<boolean> {
    console.log("unexpected guarded await");
    return true;
}

async function selectGuardedCase(subject: boolean): Promise<void> {
    switch (subject) {
        case false && await tracedCase(): {
            console.log("and bypass selected");
            break;
        }
        case true || await tracedCase(): {
            console.log("or bypass selected");
            break;
        }
        default: {
            console.log("guarded default");
        }
    }
}

async function nestedIfInSwitch(selected: boolean, branch: boolean): Promise<void> {
    switch (selected) {
        case true: {
            console.log("nested clause entered");
            if (branch) {
                const value: i32 = await Promise.resolve<i32>(7);
                await Promise.resolve<void>();
                console.log(value);
            } else {
                console.log("nested clause bypassed");
            }
            console.log("nested clause continued");
            break;
        }
        default: {
            console.log("nested clause default");
        }
    }
}

async function bothNestedBranchesInSwitch(branch: boolean): Promise<void> {
    switch (true) {
        case true: {
            if (branch) {
                await Promise.resolve<void>();
            } else {
                await Promise.resolve<void>();
            }
            console.log(branch ? "nested then resumed" : "nested else resumed");
            break;
        }
    }
}

interface OwnedProfile {
    name: string;
    aliases: string[];
}

function returnedString(): string {
    return "returned";
}

function returnedLabels(): string[] {
    return ["owned", "labels"];
}

function returnedProfile(): OwnedProfile {
    return { name: "profile", aliases: ["first", "second"] };
}

async function preserveFunctionReturns(): Promise<void> {
    let label: string = returnedString();
    let labels: string[] = returnedLabels();
    const profile: OwnedProfile = returnedProfile();
    await Promise.resolve<void>();
    label = returnedString();
    labels = returnedLabels();
    console.log(label);
    console.log(labels[1]);
    console.log(profile.aliases[0]);
}

async function nestedWhileInSwitch(): Promise<void> {
    let index: i32 = 0;
    switch (true) {
        case true: {
            while (index < 4) {
                await Promise.resolve<void>();
                index = index + 1;
                if (index === 2) {
                    continue;
                }
                console.log(index);
                if (index === 3) {
                    break;
                }
            }
            console.log("nested while continued");
            break;
        }
    }
}

async function nestedDoWhileInSwitch(): Promise<void> {
    let index: i32 = 0;
    switch (true) {
        case true: {
            do {
                await Promise.resolve<void>();
                index = index + 1;
                if (index < 2) {
                    continue;
                }
                console.log(index);
            } while (index < 2);
            console.log("nested do continued");
            break;
        }
    }
}

async function nestedForInSwitch(): Promise<void> {
    switch (true) {
        case true: {
            for (let index: i32 = 0; index < 4; index = index + 1) {
                await Promise.resolve<void>();
                if (index < 2) {
                    continue;
                }
                console.log(index);
                break;
            }
            console.log("nested for continued");
            break;
        }
    }
}

async function nestedForOfInSwitch(): Promise<void> {
    const values: i32[] = [1, 2, 3, 4];
    switch (true) {
        case true: {
            for (const value of values) {
                await Promise.resolve<void>();
                if (value === 1) {
                    continue;
                }
                console.log(value);
                if (value === 3) {
                    break;
                }
            }
            console.log("nested for-of continued");
            break;
        }
    }
}

async function nestedSwitches(): Promise<void> {
    if (true) {
        switch ("if") {
            case "if": {
                await Promise.resolve<void>();
                console.log("nested if");
                break;
            }
        }
    }
    let whileIndex: i32 = 0;
    while (whileIndex < 2) {
        switch (whileIndex) {
            case 0: {
                await Promise.resolve<void>();
                break;
            }
            default: {
                await Promise.resolve<void>();
            }
        }
        whileIndex = whileIndex + 1;
    }
    console.log("while done");
    let doIndex: i32 = 0;
    do {
        switch (doIndex) {
            default: {
                await Promise.resolve<void>();
                break;
            }
        }
        doIndex = doIndex + 1;
    } while (doIndex < 2);
    console.log("do done");
    for (let forIndex: i32 = 0; forIndex < 2; forIndex = forIndex + 1) {
        switch (forIndex) {
            default: {
                await Promise.resolve<void>();
                break;
            }
        }
    }
    const values: i32[] = [3, 4];
    let forOfTotal: i32 = 0;
    for (const value of values) {
        switch (value) {
            default: {
                await Promise.resolve<void>();
                forOfTotal = forOfTotal + value;
                break;
            }
        }
    }
    console.log(forOfTotal);
    console.log("nested loops");
    try {
        switch ("try") {
            case await Promise.reject<string>({ message: "nested switch rejected", code: "ERR_NESTED_SWITCH_REJECTED" }): {
                break;
            }
            default: {
                break;
            }
        }
    } catch (error: Error) {
        console.log(error.message);
    } finally {
        console.log("nested finally");
    }
}

async function main(): Promise<void> {
    await choose("stop");
    await choose("await");
    await choose("tail");
    await choose("missing");
    await subjectOnly();
    await ownSelectedPayload();
    await selectAwaitedCase("sync");
    await selectAwaitedCase("second");
    await selectAwaitedCase("missing");
    await selectExpressionCase(3);
    await selectExpressionCase(5);
    await selectExpressionCase(9);
    await selectOnlyMultiAwaitCase(13);
    await selectOnlyMultiAwaitCase(12);
    await selectOwnedExpressionCase("OWNED");
    await selectGuardedCase(false);
    await selectGuardedCase(true);
    await nestedIfInSwitch(true, true);
    await nestedIfInSwitch(true, false);
    await nestedIfInSwitch(false, true);
    await bothNestedBranchesInSwitch(true);
    await bothNestedBranchesInSwitch(false);
    await preserveFunctionReturns();
    await nestedWhileInSwitch();
    await nestedDoWhileInSwitch();
    await nestedForInSwitch();
    await nestedForOfInSwitch();
    await nestedSwitches();
    try {
        await rejectSelectedCase();
    } catch (error: Error) {
        console.log(error.message);
    }
    try {
        await rejectSubject();
    } catch (error: Error) {
        console.log(error.message);
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
        "subject\n7\nsubject\n123\nsubject\n3\nsubject\n4123\n22\nowned switch payload\nsync selected\nfirst\nsecond\nsecond selected\nfirst\nsecond\ncase default\nexpression selected\nexpression five\nexpression default\nsingle multi-await selected\nsingle multi-await default\nowned expression selected\nand bypass selected\nor bypass selected\nnested clause entered\n7\nnested clause continued\nnested clause entered\nnested clause bypassed\nnested clause continued\nnested clause default\nnested then resumed\nnested else resumed\nreturned\nlabels\nfirst\n1\n3\nnested while continued\n2\nnested do continued\n2\nnested for continued\n2\n3\nnested for-of continued\nnested if\nwhile done\ndo done\n7\nnested loops\nnested switch rejected\nnested finally\nswitch rejected\nsubject rejected\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn rejected_try_can_suspend_repeatedly_inside_catch_and_run_finally_once() {
    let project = AsyncProject::new(
        r#"async function flow(): Promise<void> {
    try {
        await Promise.reject<void>({ message: "boom", code: "ERR_BOOM" });
    } catch (error: Error) {
        const first: string = await Promise.resolve<string>("first");
        const second: string = await Promise.resolve<string>(first);
        console.log(error.message);
        console.log(second);
    } finally {
        const finalValue: string = await Promise.resolve<string>("finally");
        await Promise.resolve();
        console.log(finalValue);
    }
}

async function main(): Promise<void> {
    await flow();
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
        "boom\nfirst\nfinally\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn synchronous_try_can_enter_a_suspending_catch_and_run_finally_once() {
    let project = AsyncProject::new(
        r#"function failSynchronously(): void throws Error {
    const error: Error = { message: "sync failure", code: "ERR_SYNC_FAILURE" };
    throw error;
}

async function catchStep(label: string): Promise<string> {
    return label;
}

async function maybeReject(shouldReject: boolean): Promise<void> throws Error {
    if (shouldReject) {
        const error: Error = { message: "catch failure", code: "ERR_CATCH_FAILURE" };
        throw error;
    }
}

async function run(
    returnEarly: boolean,
    shouldFail: boolean,
    rejectCatch: boolean,
): Promise<void> throws Error {
    try {
        if (returnEarly) {
            return;
        }
        if (shouldFail) {
            failSynchronously();
        }
        console.log("try completed");
    } catch (error: Error) {
        const recovered: string = await catchStep("catch resumed");
        console.log(error.message);
        console.log(recovered);
        await maybeReject(rejectCatch);
        console.log("catch completed");
    } finally {
        console.log("finally");
    }
    console.log("after");
}

async function main(): Promise<void> {
    try {
        await run(true, false, false);
    } catch (error: Error) {
        console.log("unexpected", error.message);
    }
    try {
        await run(false, false, false);
    } catch (error: Error) {
        console.log("unexpected", error.message);
    }
    try {
        await run(false, true, false);
    } catch (error: Error) {
        console.log("unexpected", error.message);
    }
    try {
        await run(false, true, true);
    } catch (error: Error) {
        console.log(error.message);
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
        "finally\ntry completed\nfinally\nafter\nsync failure\ncatch resumed\ncatch completed\nfinally\nafter\nsync failure\ncatch resumed\nfinally\ncatch failure\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_try_can_enter_a_suspending_catch() {
    let project = AsyncProject::new(
        r#"async function step(shouldFail: boolean): Promise<void> throws Error {
    if (shouldFail) {
        const error: Error = { message: "if failure", code: "ERR_IF_FAILURE" };
        throw error;
    }
}

async function recover(): Promise<string> {
    return "catch resumed";
}

async function run(flag: boolean): Promise<void> {
    try {
        if (flag) {
            await step(true);
        } else {
            await step(false);
        }
    } catch (error: Error) {
        const message: string = await recover();
        console.log(error.message);
        console.log(message);
        await Promise.resolve();
        console.log("catch completed");
    } finally {
        await Promise.resolve();
        console.log("finally");
    }
    console.log("after");
}

async function main(): Promise<void> {
    await run(false);
    await run(true);
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
        "finally\nafter\nif failure\ncatch resumed\ncatch completed\nfinally\nafter\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_switch_try_can_suspend_in_catch_and_finally() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function fail(): Promise<void> throws Error {
    const error: Error = { message: "boom", code: "ERR_BOOM" };
    throw error;
}

async function run(flag: boolean): Promise<void> {
    try {
        switch (flag) {
            case true: { await fail(); break; }
            default: { await pause(); break; }
        }
    } catch (error: Error) {
        await pause();
        console.log(error.message);
    } finally {
        await pause();
        console.log("cleanup");
    }
}

async function main(): Promise<void> {
    await run(false);
    await run(true);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "cleanup\nboom\ncleanup\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_switch_try_routes_return_through_suspending_finally() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function fail(): Promise<void> throws Error {
    const error: Error = { message: "boom", code: "ERR_BOOM" };
    throw error;
}

async function run(succeed: boolean): Promise<i32> {
    try {
        switch (succeed) {
            case true: { await pause(); break; }
            default: { await fail(); break; }
        }
        return 42;
    } catch (error: Error) {
        await pause();
        console.log(error.message);
        return 0;
    } finally {
        await pause();
        console.log("cleanup");
    }
}

async function main(): Promise<void> {
    const succeeded = await run(true);
    console.log(succeeded);
    const recovered = await run(false);
    console.log(recovered);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "cleanup\n42\nboom\ncleanup\n0\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_switch_try_routes_throw_into_resumable_catch() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function run(): Promise<void> {
    try {
        switch (true) {
            case true: { await pause(); break; }
        }
        throw "boom".toUpperCase();
    } catch (error: string) {
        console.log("catch");
        await pause();
        console.log(error);
    } finally {
        await pause();
        console.log("cleanup");
    }
}

async function main(): Promise<void> {
    await run();
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "catch\nBOOM\ncleanup\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_switch_try_routes_catch_rethrow_through_suspending_finally() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function run(): Promise<void> throws string {
    try {
        switch (true) {
            case true: { await pause(); break; }
        }
        throw "boom".toUpperCase();
    } catch (error: string) {
        await pause();
        console.log("rethrow");
        throw error;
    } finally {
        await pause();
        console.log("cleanup");
    }
}

async function main(): Promise<void> {
    try {
        await run();
    } catch (error: string) {
        console.log("outer");
        console.log(error);
    }
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "rethrow\ncleanup\nouter\nBOOM\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_try_runs_a_suspending_finally_once() {
    let project = AsyncProject::new(
        r#"async function step(shouldFail: boolean): Promise<void> throws Error {
    if (shouldFail) {
        const error: Error = { message: "if failure", code: "ERR_IF_FAILURE" };
        throw error;
    }
}

async function run(shouldFail: boolean): Promise<void> {
    try {
        if (shouldFail) {
            await step(true);
        } else {
            await step(false);
        }
    } catch (error: Error) {
        console.log(error.message);
    } finally {
        const first: string = await Promise.resolve<string>("first cleanup");
        console.log(first);
        await Promise.resolve();
        console.log("finally completed");
    }
    console.log("after");
}

async function main(): Promise<void> {
    await run(false);
    await run(true);
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
        "first cleanup\nfinally completed\nafter\nif failure\nfirst cleanup\nfinally completed\nafter\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn async_finally_preserves_pending_rejection_and_overrides_it_when_finally_rejects() {
    let project = AsyncProject::new(
        r#"async function preserve(): Promise<void> {
    try {
        await Promise.reject<void>({ message: "try-failed", code: "ERR_TRY_FAILED" });
    } catch (error: Error) {
        await Promise.reject<void>({ message: "catch-failed", code: "ERR_CATCH_FAILED" });
    } finally {
        await Promise.resolve();
        console.log("preserve-cleanup");
    }
}

async function overrideFailure(): Promise<void> {
    try {
        await Promise.reject<void>({ message: "try-failed", code: "ERR_TRY_FAILED" });
    } catch (error: Error) {
        await Promise.reject<void>({ message: "catch-failed", code: "ERR_CATCH_FAILED" });
    } finally {
        await Promise.reject<void>({ message: "finally-failed", code: "ERR_FINALLY_FAILED" });
    }
}

async function main(): Promise<void> {
    try {
        await preserve();
    } catch (error: Error) {
        console.log(error.message);
    }
    try {
        await overrideFailure();
    } catch (error: Error) {
        console.log(error.message);
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
        "preserve-cleanup\ncatch-failed\nfinally-failed\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn await_inside_an_if_within_a_try_routes_rejection_to_catch_and_runs_finally() {
    let project = AsyncProject::new(
        r#"function step(shouldFail: boolean): Promise<void> {
    return shouldFail ? Promise.reject<void>({ message: "boom", code: "ERR_BOOM" }) : Promise.resolve();
}

async function run(shouldFail: boolean): Promise<void> {
    try {
        if (shouldFail) {
            await step(true);
        } else {
            await step(false);
        }
    } catch (error: Error) {
        console.log(error.message);
    } finally {
        console.log("cleanup");
    }
    console.log("done");
}

async function main(): Promise<void> {
    await run(false);
    await run(true);
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
    // run(false): the if is skipped, finally runs, then done.
    // run(true): the awaited step rejects, the catch handles it, finally runs, done.
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "cleanup\ndone\nboom\ncleanup\ndone\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn owned_optional_payloads_resolve_with_and_without_a_value() {
    // Two crashes lived here behind a compile-only test: widening a string literal
    // into `T?` released the literal (`free` on static storage), and a bare `return;`
    // resolved with a void payload the awaiting side then dereferenced. Both paths
    // only fail when the program actually runs, so this test runs it.
    let project = AsyncProject::new(
        r#"interface User {
    name: string;
}

async function literal(): Promise<string?> {
    return "hi";
}

async function owned(name: string): Promise<string?> {
    return name.toUpperCase();
}

async function empty(): Promise<string?> {
    return;
}

async function user(value?: User): Promise<User?> {
    return value;
}

async function main(): Promise<void> {
    console.log(await literal() ?? "none");
    console.log(await owned("ada") ?? "none");
    console.log(await empty() ?? "none");
    const present = await user({ name: "ada".toUpperCase() });
    console.log(present?.name ?? "none");
    const absent = await user();
    console.log(absent?.name ?? "none");
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
        "hi\nADA\nnone\nADA\nnone\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn recursively_owned_payloads_transfer_once_through_delegation_and_discard() {
    let project = AsyncProject::new(
        r#"interface Metadata {
    label: string;
    aliases: string[];
    note?: string;
}

interface Result {
    status: i32;
    metadata: Metadata;
}

async function source(label: string): Promise<Result> {
    return {
        status: 200,
        metadata: {
            label: label.toUpperCase(),
            aliases: "ready,done".split(","),
            note: label,
        },
    };
}

async function delegated(label: string): Promise<Result> {
    return await source(label);
}

async function adopted(label: string): Promise<Result> {
    return delegated(label);
}

async function main(): Promise<void> {
    const result: Result = await adopted("value");
    console.log("checkpoint recursive: adopted result received");
    console.log(result.status);
    console.log(result.metadata.label);
    console.log(result.metadata.aliases[1]);
    console.log(result.metadata.note ?? "missing");
    console.log("checkpoint recursive: before discarded result");
    await source("discarded");
    console.log("checkpoint recursive: discarded result received");
    console.log("discarded owned payload");
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
        "program failed ({}):\nstdout:\n{}\nstderr:\n{}",
        process_status_description(&output.status),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_unexpected_stderr(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "checkpoint recursive: adopted result received\n200\nVALUE\ndone\nvalue\ncheckpoint recursive: before discarded result\ncheckpoint recursive: discarded result received\ndiscarded owned payload\n"
    );
}

#[test]
fn promise_combinators_preserve_struct_payload_ownership() {
    let project = AsyncProject::new(
        r#"interface Profile {
    name: string;
    aliases: string[];
}

async function profile(name: string): Promise<Profile> {
    return { name: name.toUpperCase(), aliases: name.split("") };
}

async function main(): Promise<void> {
    console.log("checkpoint combinators: start");
    const all: Profile[] = await Promise.all<Profile>([
        profile("ada"),
        profile("grace"),
    ]);
    console.log("checkpoint combinators: all fulfilled");
    console.log(all[0].name);
    console.log(all[1].aliases[0]);

    const settled = await Promise.allSettled<Profile>([
        profile("margaret"),
        Promise.reject<Profile>({ message: "no profile", code: "ERR_NO_PROFILE" }),
    ]);
    console.log("checkpoint combinators: allSettled fulfilled");
    await Promise.resolve<void>();
    console.log("checkpoint combinators: resolved promise fulfilled");
    console.log(settled.length);

    const first: Profile = await Promise.race<Profile>([profile("linus")]);
    console.log("checkpoint combinators: race fulfilled");
    console.log(first.name);

    const any: Profile = await Promise.any<Profile>([profile("barbara")]);
    console.log("checkpoint combinators: any fulfilled");
    console.log(any.name);

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
        "program failed ({}):\nstdout:\n{}\nstderr:\n{}",
        process_status_description(&output.status),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_unexpected_stderr(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "checkpoint combinators: start\ncheckpoint combinators: all fulfilled\nADA\ng\ncheckpoint combinators: allSettled fulfilled\ncheckpoint combinators: resolved promise fulfilled\n2\ncheckpoint combinators: race fulfilled\nLINUS\ncheckpoint combinators: any fulfilled\nBARBARA\n"
    );
}

#[test]
fn async_parameter_forms_survive_suspension_with_recursive_ownership() {
    let project = AsyncProject::new(
        r#"interface Profile {
    name: string;
    aliases: string[];
}

async function pause(): Promise<void> {
    return;
}

async function defaulted(count: i32 = 2): Promise<i32> {
    await pause();
    return count;
}

async function optionalProfile(value?: Profile): Promise<string> {
    await pause();
    return value?.name ?? "missing";
}

async function defaultProfile(
    value: Profile = { name: "Default", aliases: ["D"] },
): Promise<string> {
    await pause();
    return value.aliases[0];
}

async function arrayPattern([left, right]: string[]): Promise<string> {
    await pause();
    return left + right;
}

async function objectPattern({ name, aliases }: Profile): Promise<string> {
    await pause();
    return name + aliases[0];
}

async function stringRest(...values: string[]): Promise<string> {
    await pause();
    return values[0] + values[1];
}

async function objectRest(...values: Profile[]): Promise<string> {
    await pause();
    return values[0].name + values[1].aliases[0];
}

async function main(): Promise<void> {
    console.log(await defaulted());
    console.log(await optionalProfile());
    console.log(await optionalProfile({ name: "Ada", aliases: ["A"] }));
    console.log(await defaultProfile());
    console.log(await arrayPattern(["A", "B"]));
    console.log(await objectPattern({ name: "Ada", aliases: ["!"] }));
    console.log(await stringRest("A", "B"));
    console.log(await objectRest(
        { name: "Ada", aliases: ["A"] },
        { name: "Grace", aliases: ["!"] },
    ));
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
        "2\nmissing\nAda\nD\nAB\nAda!\nAB\nAda!\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn multiple_async_closures_keep_independent_owned_capture_frames() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    const firstPrefix: string = "first".toUpperCase();
    const secondPrefix: string = "second".toUpperCase();
    const first = async (suffix: string): Promise<string> => {
        await pause();
        return firstPrefix + suffix;
    };
    const second = async (suffix: string): Promise<string> => {
        await pause();
        return secondPrefix + suffix;
    };
    console.log(await first("!"));
    console.log(await second("?"));
    console.log(await first("."));
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
        "FIRST!\nSECOND?\nFIRST.\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn cooperative_cancellation_propagates_once_and_cleans_owned_frame_state() {
    let project = AsyncProject::new(
        r#"import { cancel, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const owned: string = "frame cleanup".toUpperCase();
    const operation: Promise<void> = sleepAsync(1000);
    console.log(cancel(operation, "Cancelled by test"));
    console.log(cancel(operation, "Ignored"));
    try {
        await operation;
    } catch (error: Error) {
        console.log(error.message);
    }
    console.log(owned);
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
        "true\nfalse\nCancelled by test\nFRAME CLEANUP\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn spawn_runs_a_task_concurrently_and_join_resolves_its_result() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        return 42;
    });
    const value = await handle.join();
    console.log(value);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn spawn_runs_multiple_tasks_in_parallel_rather_than_sequentially() {
    // Two tasks each sleeping 2.5s, joined one after the other, finish in close
    // to 2.5s total when `spawn` schedules them concurrently, and close to 5s
    // when execution is sequential. The elapsed time includes process startup,
    // so the 4.2s limit leaves room for sanitizer overhead while preserving an
    // 800ms gap from the sequential schedule.
    let project = AsyncProject::new(
        r#"import { spawn, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const first = spawn(async (): Promise<i32> => {
        await sleepAsync(2500);
        return 1;
    });
    const second = spawn(async (): Promise<i32> => {
        await sleepAsync(2500);
        return 2;
    });
    const a = await first.join();
    const b = await second.join();
    console.log(a + b);
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
    let started = std::time::Instant::now();
    let output = project.run();
    let elapsed = started.elapsed();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "3\n");
    assert!(
        elapsed < std::time::Duration::from_millis(4200),
        "two 2.5s tasks took {elapsed:?}; spawn does not appear to run them concurrently"
    );
}

#[test]
fn spawn_rejects_a_task_that_does_not_return_a_promise() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(() => 42);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for a non-async task passed to `spawn`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3030") || stderr.contains("E3030"),
        "expected `E3030` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn join_handle_rejects_unknown_methods() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        return 42;
    });
    handle.cancel();
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for an unknown `JoinHandle` method"
    );
    assert!(
        String::from_utf8_lossy(&build.stderr).contains("E0599")
            || String::from_utf8_lossy(&build.stdout).contains("E0599"),
        "expected E0599 in build output:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
}

#[test]
fn join_handle_abort_cancels_the_task_before_it_completes() {
    let project = AsyncProject::new(
        r#"import { spawn, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        await sleepAsync(5000);
        return 42;
    });
    handle.abort();
    try {
        const value = await handle.join();
        console.log(value);
    } catch (error: Error) {
        console.log(error.message);
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
    let started = std::time::Instant::now();
    let output = project.run();
    let elapsed = started.elapsed();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Aborted by caller\n"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(3000),
        "abort() did not cancel the 5s sleep before it completed: took {elapsed:?}"
    );
}

// `JoinHandle<T>.join()` consumes the handle in codegen (forwards its single
// `*mut TyrusPromise` reference instead of retaining a second one, retaining
// left the original reference from `spawn()` itself leaked, confirmed with
// `valgrind --leak-check=full` on a 5,000-iteration spawn+join loop before
// this fix landed: heap usage at exit grew with the iteration count instead
// of staying flat). These two tests are the compile-time half of that fix:
// a `JoinHandle<T>` can only be joined once.

#[test]
fn rejects_joining_the_same_handle_twice() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        return 42;
    });
    const first = await handle.join();
    const second = await handle.join();
    console.log(first + second);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for joining the same `JoinHandle` twice"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3033") || stderr.contains("E3033"),
        "expected `E3033` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rejects_aborting_a_handle_after_it_was_joined() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        return 42;
    });
    const value = await handle.join();
    handle.abort();
    console.log(value);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for aborting a `JoinHandle` after it was joined"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3033") || stderr.contains("E3033"),
        "expected `E3033` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rejects_awaiting_spawn_directly_without_holding_the_join_handle() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const value = await spawn(async (): Promise<i32> => {
        return 42;
    });
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for `await spawn(...)` directly"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3031") || stderr.contains("E3031"),
        "expected `E3031` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rejects_awaiting_spawn_join_chained_in_the_same_expression() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const value = await spawn(async (): Promise<i32> => {
        return 42;
    }).join();
    console.log(value);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for `await spawn(...).join()` chained in one expression"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3031") || stderr.contains("E3031"),
        "expected `E3031` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rejects_a_discarded_join_handle_the_same_way_as_a_discarded_promise() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    spawn(async (): Promise<i32> => {
        return 42;
    });
    console.log("done");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for a discarded `JoinHandle<T>`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3005") || stderr.contains("E3005"),
        "expected `E3005` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn spawn_propagates_a_thrown_task_error_through_join() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> throws string => {
        throw "task failed";
    });
    try {
        const value = await handle.join();
        console.log(value);
    } catch (error: string) {
        console.log(error);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "task failed\n");
}

// Fase 4 of tasks.md (captures, ownership, borrow checker) turned out to need
// no new implementation: `spawn`'s task is an ordinary escaping `async` arrow
// function argument, so it already goes through the same async-capture
// analysis every other escaping async closure does (`async-unsupported-
// capture`/`async-borrow-across-await` in `type_checker.rs` for references,
// `E0382` for owned values reused after the move). These tests exist to
// cover that behavior specifically through `spawn`, not to introduce it.

#[test]
fn spawn_rejects_capturing_a_local_reference() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    let owner: string = "Ada";
    const borrowed: &string = &owner;
    const handle = spawn(async (): Promise<void> => {
        console.log(borrowed);
    });
    await handle.join();
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for capturing a local reference in a spawn task"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3085") || stderr.contains("E3085"),
        "expected `E3085` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn spawn_accepts_moving_an_owned_capture_with_no_later_use() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const owned: string = "Ada".toUpperCase();
    const handle = spawn(async (): Promise<void> => {
        console.log(owned);
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "ADA\n");
}

#[test]
fn spawn_rejects_reusing_an_owned_capture_after_it_moved_into_the_task() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const owned: string = "Ada".toUpperCase();
    const handle = spawn(async (): Promise<void> => {
        console.log(owned);
    });
    console.log(owned);
    await handle.join();
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for reusing an owned value after it moved into a spawn task"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E0382") || stderr.contains("E0382"),
        "expected `E0382` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("value moved here") || stderr.contains("value moved here"),
        "expected related information pointing at the move site:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn spawn_accepts_copy_captures_reused_after_the_task() {
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const count: i32 = 42;
    const handle = spawn(async (): Promise<void> => {
        console.log(count);
    });
    console.log(count);
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\n42\n");
}

#[test]
fn awaited_payload_still_widens_into_an_optional_binding() {
    // Gating owned *optional payloads* must not gate the documented case where the
    // payload itself is owned and only the binding is optional. This one runs, so a
    // regression here would be a crash rather than a compile error.
    let project = AsyncProject::new(
        r#"async function load(): Promise<string> {
    return "hi";
}

async function main(): Promise<void> {
    const value: string? = await load();
    console.log(value ?? "none");
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hi\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn await_in_both_branches_of_an_if_selects_the_branch_and_converges() {
    let project = AsyncProject::new(
        r#"function step(shouldFail: boolean): Promise<void> {
    return shouldFail ? Promise.reject<void>({ message: "boom", code: "ERR_BOOM" }) : Promise.resolve();
}

async function run(flag: boolean): Promise<void> {
    if (flag) {
        console.log("then-side");
        await step(false);
    } else {
        console.log("else-side");
        await step(true);
    }
    console.log("after");
}

async function main(): Promise<void> {
    await run(true);
    try {
        await run(false);
    } catch (error: Error) {
        console.log(error.message);
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
    // run(true): then-side runs, its await fulfills, "after" runs.
    // run(false): else-side runs, its await rejects, propagating to main's catch.
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "then-side\nafter\nelse-side\nboom\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn await_in_both_if_branches_can_assign_the_shared_continuation_value() {
    let project = AsyncProject::new(
        r#"async function select(flag: boolean): Promise<i32> {
    let value: i32 = 0;
    if (flag) {
        value = await Promise.resolve<i32>(7);
    } else {
        value = await Promise.resolve<i32>(9);
    }
    return value;
}

async function main(): Promise<void> {
    console.log(await select(true));
    console.log(await select(false));
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "7\n9\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn await_in_both_if_branches_preserves_branch_local_continuations() {
    let project = AsyncProject::new(
        r#"async function value(input: i32): Promise<i32> {
    return input;
}

async function select(flag: boolean): Promise<void> {
    if (flag) {
        const left: i32 = await value(7);
        console.log("left", left);
    } else {
        const right: i32 = await value(9);
        console.log("right", right);
    }
    console.log("after");
}

async function main(): Promise<void> {
    await select(true);
    await select(false);
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
        "left 7\nafter\nright 9\nafter\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn discarded_awaits_in_both_if_branches_may_have_different_payload_types() {
    let project = AsyncProject::new(
        r#"async function select(flag: boolean): Promise<void> {
    if (flag) {
        await Promise.resolve<i32>(7);
        console.log("number discarded");
    } else {
        await Promise.resolve<string>("unused");
        console.log("string discarded");
    }
}

async function main(): Promise<void> {
    await select(true);
    await select(false);
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
        "number discarded\nstring discarded\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn both_if_branches_support_independent_multi_suspend_paths() {
    let project = AsyncProject::new(
        r#"let conditionChecks: i32 = 0;

function choose(flag: boolean): boolean {
    conditionChecks += 1;
    return flag;
}

async function select(flag: boolean): Promise<void> {
    if (choose(flag)) {
        const number: i32 = await Promise.resolve<i32>(7);
        await Promise.resolve();
        const text: string = await Promise.resolve<string>("then-owned");
        console.log(number);
        console.log(text);
    } else {
        const text: string = await Promise.resolve<string>("else-owned");
        const number: i32 = await Promise.resolve<i32>(9);
        await Promise.resolve();
        console.log(text);
        console.log(number);
    }
    console.log("joined");
}

async function nested(flag: boolean): Promise<i32> {
    let total: i32 = 0;
    while (total < 1) {
        await Promise.resolve();
        if (flag) {
            total = await Promise.resolve<i32>(3);
            await Promise.resolve();
        } else {
            await Promise.resolve<string>("discarded");
            total = await Promise.resolve<i32>(4);
            await Promise.resolve();
        }
        await Promise.resolve();
        break;
    }
    return total;
}

async function recovered(flag: boolean): Promise<void> {
    let iteration: i32 = 0;
    while (iteration < 1) {
        iteration += 1;
        try {
            if (flag) {
                await Promise.resolve();
                await Promise.reject<void>({ message: "branch failure", code: "ERR_BRANCH_FAILURE" });
            } else {
                await Promise.resolve<string>("kept separate");
                await Promise.resolve();
            }
        } catch (error: Error) {
            console.log(error.message);
        } finally {
            await Promise.resolve();
            console.log("branch cleanup");
        }
    }
}

async function main(): Promise<void> {
    await select(true);
    await select(false);
    console.log(conditionChecks);
    console.log(await nested(true));
    console.log(await nested(false));
    await recovered(true);
    await recovered(false);
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
        "7\nthen-owned\njoined\nelse-owned\n9\njoined\n2\n3\n4\nbranch failure\nbranch cleanup\nbranch cleanup\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn awaited_if_conditions_select_after_resumption_and_propagate_rejection() {
    let project = AsyncProject::new(
        r#"async function enabled(flag: boolean): Promise<boolean> {
    return flag;
}

async function value(input: i32): Promise<i32> {
    return input;
}

async function rejectedCondition(): Promise<boolean> throws Error {
    const error: Error = { message: "condition failed", code: "ERR_CONDITION_FAILED" };
    throw error;
}

async function select(flag: boolean, input: i32): Promise<void> {
    if (await enabled(flag)) {
        console.log("enabled");
    } else {
        console.log("disabled");
    }

    if ((await value(input)) > 0) {
        console.log("positive");
    } else {
        console.log("not positive");
    }
}

async function failWhileSelecting(): Promise<void> throws Error {
    if (await rejectedCondition()) {
        console.log("unreachable");
    }
}

async function main(): Promise<void> {
    await select(true, 7);
    await select(false, -1);
    try {
        await failWhileSelecting();
    } catch (error: Error) {
        console.log(error.message);
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
        "enabled\npositive\ndisabled\nnot positive\ncondition failed\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn short_circuit_loop_guards_skip_the_awaited_dependency() {
    let project = AsyncProject::new(
        r#"async function observed(value: i32): Promise<i32> {
    console.log("awaited");
    return value;
}

const runGuardedLoops = async (): Promise<void> => {
    let whileValue: i32 = -1;
    while (whileValue >= 0 && await observed(whileValue) < 3) {
        whileValue += 1;
    }
    console.log("while-finished");

    do {
        console.log("do-body");
    } while (false && await observed(0) < 3);

    for (let index: i32 = -1; index >= 0 && await observed(index) < 3; index += 1) {
        console.log("for-body");
    }
    console.log("for-finished");

    let oneIteration: i32 = 0;
    while (oneIteration >= 0 && await observed(oneIteration) < 3) {
        oneIteration = -1;
    }
    console.log("done");
};

async function main(): Promise<void> {
    await runGuardedLoops();
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
        "while-finished\ndo-body\nfor-finished\nawaited\ndone\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_inside_while_preserves_break_and_continue() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let index: i32 = 0;
    let total: i32 = 0;
    while (index < 6) {
        index += 1;
        if (index % 2 === 0) {
            await pause();
            if (index === 2) {
                continue;
            }
        }
        total += index;
        if (index === 5) {
            break;
        }
    }
    console.log(total);
    console.log(index);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "13\n5\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_inside_do_while_preserves_break_and_continue() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function main(): Promise<void> {
    let index: i32 = 0;
    let total: i32 = 0;
    do {
        index += 1;
        if (index % 2 === 0) {
            await pause();
            if (index === 2) { continue; }
        }
        total += index;
        if (index === 5) { break; }
    } while (index < 6);
    console.log(total);
    console.log(index);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "13\n5\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_inside_for_preserves_break_continue_and_update() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function main(): Promise<void> {
    let total: i32 = 0;
    let observed: i32 = 0;
    for (let index: i32 = 0; index < 6; index += 1) {
        observed = index;
        if (index % 2 === 0) {
            await pause();
            if (index === 0) { continue; }
        }
        total += index;
        if (index === 4) { break; }
    }
    console.log(total);
    console.log(observed);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n4\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_if_inside_for_of_preserves_break_continue_and_iteration() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function main(): Promise<void> {
    const values: i32[] = [1, 2, 3, 4, 5];
    let total: i32 = 0;
    let observed: i32 = 0;
    for (const value of values) {
        observed = value;
        if (observed % 2 === 0) {
            await pause();
            if (observed === 2) { continue; }
        }
        if (observed === 4) { break; }
        total += observed;
    }
    console.log(total);
    console.log(observed);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "4\n4\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn suspending_loop_branches_preserve_return_and_throw_completion() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function selectValue(): Promise<i32> {
    let index: i32 = 0;
    while (index < 2) {
        index += 1;
        if (index === 1) {
            await pause();
            return 7;
        }
    }
    return 0;
}

async function failFromLoop(): Promise<void> throws string {
    for (let index: i32 = 0; index < 2; index += 1) {
        if (index === 1) {
            await pause();
            throw "loop failure";
        }
    }
}

async function main(): Promise<void> {
    const selected: i32 = await selectValue();
    console.log(selected);
    try {
        await failFromLoop();
    } catch (error: string) {
        console.log(error);
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
        "7\nloop failure\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn compound_string_assignment_replaces_owned_async_frame_storage() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let message: string = "before".toUpperCase();
    await pause();
    message += " after";
    await pause();
    message += " done".toUpperCase();
    console.log(message);
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
        "BEFORE after DONE\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn awaited_string_assignment_transfers_payload_ownership_into_the_frame() {
    let project = AsyncProject::new(
        r#"async function message(value: string): Promise<string> {
    return value;
}

async function main(): Promise<void> {
    let value: string = "static";
    value = await message("first".toUpperCase());
    console.log(value);
    value = await message("second".toUpperCase());
    await Promise.resolve<void>();
    console.log(value);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "FIRST\nSECOND\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn indexed_string_array_mutation_normalizes_and_replaces_owned_elements() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let labels: string[] = ["static", "array"];
    await pause();
    labels[0] = "first".toUpperCase();
    labels[1] = labels[0];
    await pause();
    labels[0] = "final";
    console.log(labels[0]);
    console.log(labels[1]);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "final\nFIRST\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn mutable_copy_objects_survive_field_mutation_and_reassignment_across_awaits() {
    let project = AsyncProject::new(
        r#"interface Cursor {
    row: i32;
    column: i32;
    visible: boolean;
}

async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let cursor: Cursor = { row: 1, column: 2, visible: true };
    await pause();
    cursor.row = 3;
    console.log(cursor.row);
    cursor = { row: 4, column: 5, visible: false };
    await pause();
    console.log(cursor.row);
    console.log(cursor.column);
    console.log(cursor.visible);
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
        "3\n4\n5\nfalse\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn mutable_owned_objects_are_deeply_replaced_across_awaits() {
    let project = AsyncProject::new(
        r#"interface Snapshot {
    count: i32;
    label: string;
    tags: string[];
    note?: string;
}

async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    const seed: Snapshot = { count: 1, label: "first", tags: ["static"] };
    let snapshot: Snapshot = seed;
    await pause();
    console.log("checkpoint mutable: first await resumed");
    snapshot.count = 2;
    console.log(snapshot.count);
    snapshot.label = snapshot.label.toUpperCase();
    snapshot.tags = "direct,field".split(",");
    snapshot.tags.push("tail");
    snapshot.note = snapshot.label;
    console.log("checkpoint mutable: first mutation group complete");
    await pause();
    console.log("checkpoint mutable: second await resumed");
    console.log(snapshot.label);
    console.log(snapshot.tags[0]);
    console.log(snapshot.tags[1]);
    console.log(snapshot.tags[2]);
    console.log(snapshot.note ?? "missing");
    snapshot = {
        count: 3,
        label: "second".toUpperCase(),
        tags: "owned,array".split(","),
        note: "present".toUpperCase(),
    };
    console.log("checkpoint mutable: replacement complete");
    await pause();
    console.log("checkpoint mutable: third await resumed");
    console.log(snapshot.count);
    console.log(snapshot.label);
    console.log(snapshot.tags[0]);
    console.log(snapshot.tags[1]);
    console.log(snapshot.note ?? "missing");
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
        "program failed ({}):\nstdout:\n{}\nstderr:\n{}",
        process_status_description(&output.status),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_no_unexpected_stderr(&output);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "checkpoint mutable: first await resumed\n2\ncheckpoint mutable: first mutation group complete\ncheckpoint mutable: second await resumed\nFIRST\ndirect\nfield\ntail\nFIRST\ncheckpoint mutable: replacement complete\ncheckpoint mutable: third await resumed\n3\nSECOND\nowned\narray\nPRESENT\n"
    );
}

#[test]
fn string_array_push_and_unshift_clone_elements_owned_by_the_async_frame() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let labels: string[] = ["middle"];
    await pause();
    labels.push("last".toUpperCase());
    labels.unshift(labels[0]);
    labels[1] = "changed";
    await pause();
    console.log(labels[0]);
    console.log(labels[1]);
    console.log(labels[2]);
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
        "middle\nchanged\nLAST\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn string_array_pop_and_shift_transfer_owned_elements_out_of_the_async_frame() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let labels: string[] = ["first", "middle", "last"];
    await pause();
    labels.reverse();
    const last: string? = labels.pop();
    const first: string? = labels.shift();
    await pause();
    console.log(last ?? "missing");
    console.log(first ?? "missing");
    console.log(labels[0]);
    const middle: string? = labels.pop();
    const empty: string? = labels.pop();
    console.log(middle ?? "missing");
    console.log(empty ?? "empty");
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
        "first\nlast\nmiddle\nmiddle\nempty\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn remaining_string_array_mutations_preserve_async_frame_ownership() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let labels: string[] = ["charlie", "bravo", "alpha"];
    await pause();
    labels.fill("delta".toUpperCase(), 1, 2);
    labels.copyWithin(1, 0, 1);
    labels.sort();
    const removed: string[] = labels.splice(1, 1, "bravo".toUpperCase());
    await pause();
    console.log(labels[0]);
    console.log(labels[1]);
    console.log(labels[2]);
    console.log(removed[0]);
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
        "alpha\nBRAVO\ncharlie\ncharlie\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn owned_string_array_results_can_replace_a_local_across_suspensions() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let labels: string[] = ["alpha"];
    await pause();
    labels = labels.concat(["beta".toUpperCase()]);
    await pause();
    labels = labels.slice(1).toReversed().values();
    console.log(labels[0]);
    console.log(labels.length);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "BETA\n1\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn recursively_nested_control_flow_preserves_branches_loops_and_transfers() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let total: i32 = 0;
    let outer: i32 = 0;
    while (outer < 3) {
        outer += 1;
        let inner: i32 = 0;
        while (inner < 4) {
            inner += 1;
            if (inner === 2) {
                await pause();
                continue;
            }
            if (outer === 3 && inner === 3) {
                await pause();
                break;
            }
            total += outer * 10 + inner;
        }
    }
    let selected: i32 = 0;
    while (selected < 1) {
        if (total > 0) {
            const left: i32 = await Promise.resolve<i32>(7);
            selected += left;
            break;
        } else {
            const right: i32 = await Promise.resolve<i32>(9);
            selected += right;
            break;
        }
    }
    console.log(total);
    console.log(selected);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "137\n7\n");
    assert!(output.stderr.is_empty());
}

/// Regression test: a `for` loop's own loop variable used to become
/// unresolvable ("`r` not in codegen scope") once an `await` inside a
/// *nested* loop forced the outer `for` through the async recursive-control
/// codegen path. The outer `for`'s init scope (where `r` lives) was popped
/// alongside its body scope before `update` (`r++`) ran, instead of staying
/// open through it -- `while`/`do-while`/`for...of` never hit this because
/// they only ever push one scope, not two. Covers `for`+`for`, `while`+`for`
/// and `for`+`while` nesting, since the bug's presence depended on which
/// nesting level (if any) was a `for`.
#[test]
fn nested_for_loops_keep_the_outer_loop_variable_in_scope_across_an_inner_await() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let total: i32 = 0;
    for (let r: i32 = 0; r < 3; r += 1) {
        for (let i: i32 = 0; i < 2; i += 1) {
            await pause();
            total += r * 10 + i;
        }
    }
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "63\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn a_while_loop_nesting_an_inner_for_loop_keeps_the_inner_variable_in_scope_across_await() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let total: i32 = 0;
    let r: i32 = 0;
    while (r < 3) {
        for (let i: i32 = 0; i < 2; i += 1) {
            await pause();
            total += r * 10 + i;
        }
        r += 1;
    }
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "63\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn a_for_loop_nesting_an_inner_while_loop_keeps_the_outer_variable_in_scope_across_await() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    let total: i32 = 0;
    for (let r: i32 = 0; r < 3; r += 1) {
        let i: i32 = 0;
        while (i < 2) {
            await pause();
            total += r * 10 + i;
            i += 1;
        }
    }
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "63\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn recursive_async_try_preserves_return_rejection_and_finally_completion() {
    let project = AsyncProject::new(
        r#"async function step(shouldFail: boolean): Promise<void> throws Error {
    if (shouldFail) {
        const failure: Error = { message: "boom", code: "ERR_BOOM" };
        throw failure;
    }
}

async function select(shouldFail: boolean): Promise<i32> {
    let cleanupCount: i32 = 0;
    let round: i32 = 0;
    while (round < 1) {
        round += 1;
        try {
            if (round === 1) {
                await step(shouldFail);
                return 7;
            }
        } catch (error: Error) {
            console.log(error.message);
        } finally {
            cleanupCount += 1;
            console.log(cleanupCount);
        }
    }
    return -1;
}

async function main(): Promise<void> {
    console.log(await select(false));
    console.log(await select(true));
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
        "1\n7\nboom\n1\n-1\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn recursive_async_finally_preserves_all_completion_modes_and_sibling_identity() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function fail(): Promise<void> throws Error {
    const failure: Error = { message: "recursive finally failure", code: "ERR_RECURSIVE_FINALLY_FAILURE" };
    throw failure;
}

async function run(): Promise<i32> {
    let total: i32 = 0;
    let index: i32 = 0;
    while (index < 2) {
        index += 1;
        try {
            await fail();
            total += 10;
        } catch (error: Error) {
            await pause();
            total += 1;
        } finally {
            await pause();
            await pause();
            total += 100;
        }
    }
    return total;
}

async function runReturn(): Promise<i32> {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await pause();
            return 7;
        } catch (error: Error) {
            return -2;
        } finally {
            await pause();
            console.log("recursive return finally");
        }
    }
    return -1;
}

async function runTransfers(): Promise<i32> {
    let total: i32 = 0;
    let index: i32 = 0;
    while (index < 3) {
        index += 1;
        try {
            await pause();
            if (index === 1) { continue; }
            if (index === 3) { break; }
            total += index;
        } catch (error: Error) {
            total -= 100;
        } finally {
            await pause();
            total += 10;
        }
    }
    return total;
}

async function runRethrow(): Promise<void> throws Error {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await fail();
        } catch (error: Error) {
            await pause();
            throw error;
        } finally {
            await pause();
            console.log("recursive rethrow finally");
        }
    }
}

async function runSiblingFinalizers(): Promise<i32> {
    let result: i32 = 0;
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await pause();
            result = result * 10 + 1;
        } catch (error: Error) {
            result = -1;
        } finally {
            await pause();
            result = result * 10 + 2;
        }
        try {
            await pause();
            result = result * 10 + 3;
        } catch (error: Error) {
            result = -2;
        } finally {
            await pause();
            result = result * 10 + 4;
        }
    }
    return result;
}

async function main(): Promise<void> {
    console.log(await run());
    console.log(await runReturn());
    console.log(await runTransfers());
    try {
        await runRethrow();
    } catch (error: Error) {
        console.log(error.message);
    }
    console.log(await runSiblingFinalizers());
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
        "202\nrecursive return finally\n7\n32\nrecursive rethrow finally\nrecursive finally failure\n1234\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn recursive_async_catch_resumes_multiple_times_and_runs_finally_once() {
    let project = AsyncProject::new(
        r#"async function step(shouldFail: boolean): Promise<void> throws Error {
    if (shouldFail) {
        const failure: Error = { message: "recursive catch failure", code: "ERR_RECURSIVE_CATCH_FAILURE" };
        throw failure;
    }
}

async function pause(): Promise<void> { return; }

async function run(shouldFail: boolean): Promise<i32> {
    let total: i32 = 0;
    let cleanupCount: i32 = 0;
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(shouldFail);
            total += 2;
        } catch (error: Error) {
            console.log(error.message);
            await pause();
            total += 1;
            await pause();
            total += 2;
        } finally {
            cleanupCount += 1;
        }
    }
    return total + cleanupCount * 10;
}

async function runContinue(): Promise<i32> {
    let total: i32 = 0;
    let cleanupCount: i32 = 0;
    let index: i32 = 0;
    while (index < 2) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            if (index === 1) { continue; }
            total += 2;
        } finally {
            cleanupCount += 1;
        }
    }
    return total + cleanupCount * 10;
}

async function runReturn(): Promise<i32> {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            return 7;
        } finally {
            console.log("recursive return cleanup");
        }
    }
    return -1;
}

async function rejectFromCatch(): Promise<void> throws Error {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await step(true);
        } finally {
            console.log("recursive rejection cleanup");
        }
    }
}

async function rethrowFromCatch(): Promise<void> throws Error {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            throw error;
        } finally {
            console.log("recursive rethrow cleanup");
        }
    }
}

async function catchRethrowLocally(): Promise<i32> {
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            try {
                throw error;
            } catch (forwarded: Error) {
                console.log(forwarded.message);
                return 9;
            }
        } finally {
            console.log("recursive local rethrow cleanup");
        }
    }
    return -1;
}

async function runSiblingCatches(): Promise<i32> {
    let result: i32 = 0;
    let index: i32 = 0;
    while (index < 1) {
        index += 1;
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            result = result * 10 + 1;
        }
        try {
            await step(true);
        } catch (error: Error) {
            await pause();
            result = result * 10 + 2;
        }
    }
    return result;
}

async function main(): Promise<void> {
    console.log(await run(false));
    console.log(await run(true));
    console.log(await runContinue());
    console.log(await runReturn());
    try {
        await rejectFromCatch();
    } catch (error: Error) {
        console.log(error.message);
    }
    try {
        await rethrowFromCatch();
    } catch (error: Error) {
        console.log(error.message);
    }
    console.log(await catchRethrowLocally());
    console.log(await runSiblingCatches());
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
        "12\nrecursive catch failure\n13\n22\nrecursive return cleanup\n7\nrecursive rejection cleanup\nrecursive catch failure\nrecursive rethrow cleanup\nrecursive catch failure\nrecursive catch failure\nrecursive local rethrow cleanup\n9\n12\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn recursive_switch_try_preserves_dispatch_rejection_and_loop_control() {
    let project = AsyncProject::new(
        r#"async function step(shouldFail: boolean): Promise<void> throws Error {
    if (shouldFail) {
        const failure: Error = { message: "boom", code: "ERR_BOOM" };
        throw failure;
    }
}

async function run(shouldFail: boolean): Promise<i32> {
    let cleanupCount: i32 = 0;
    let total: i32 = 0;
    let index: i32 = 0;
    while (index < 4) {
        index += 1;
        try {
            switch (index) {
                case 1: { await step(shouldFail); total += 10; continue; }
                case 2: { await step(false); total += 20; break; }
                default: { await step(false); total += 1; }
            }
        } catch (error: Error) {
            console.log(error.message);
            total -= 5;
        } finally {
            cleanupCount += 1;
        }
        if (index === 3) { break; }
    }
    console.log(cleanupCount);
    return total;
}

async function sumValues(values: i32[]): Promise<i32> {
    let total: i32 = 0;
    for (const value of values) {
        try {
            switch (value) {
                case 2: { await step(false); total += value; break; }
                default: { await step(false); total += value; }
            }
        } catch (error: Error) {
            return -1;
        } finally {
            total += 0;
        }
    }
    return total;
}

async function main(): Promise<void> {
    console.log(await run(false));
    console.log(await run(true));
    const values: i32[] = [1, 2, 3];
    console.log(await sumValues(values));
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
        "3\n31\nboom\n3\n16\n6\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn recursive_sibling_switches_keep_independent_continuation_blocks() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function run(): Promise<i32> {
    let count: i32 = 0;
    let trace: i32 = 0;
    while (count < 2) {
        count += 1;
        if (count > 0) {
            switch (count) {
                case 1: { await pause(); trace = trace * 10 + 1; break; }
                default: { await pause(); trace = trace * 10 + 1; }
            }
            switch (count) {
                case 1: { await pause(); trace = trace * 10 + 2; break; }
                default: { await pause(); trace = trace * 10 + 2; }
            }
        }
    }
    return trace;
}

async function main(): Promise<void> {
    console.log(await run());
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1212\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn abrupt_completions_survive_a_suspending_finally() {
    let project = AsyncProject::new(
        r#"async function pause(): Promise<void> { return; }

async function selected(): Promise<i32> {
    try {
        return 7;
    } catch (error: Error) {
        return 0;
    } finally {
        const marker: string = await Promise.resolve<string>("cleanup");
        console.log(marker);
        await pause();
    }
}

async function transferred(): Promise<i32> {
    let total: i32 = 0;
    let index: i32 = -1;
    while (index < 3) {
        index += 1;
        try {
            if (index === 1) { continue; }
            if (index === 3) { break; }
            total += index;
        } catch (error: Error) {
            return -1;
        } finally {
            await pause();
            console.log("loop cleanup");
        }
    }
    return total;
}

async function failed(): Promise<void> throws Error {
    try {
        const failure: Error = { message: "completion failure", code: "ERR_COMPLETION_FAILURE" };
        throw failure;
    } catch (error: Error) {
        throw error;
    } finally {
        await pause();
        console.log("throw cleanup");
    }
}

async function overridden(): Promise<i32> {
    try {
        return 1;
    } catch (error: Error) {
        return 0;
    } finally {
        await pause();
        return 9;
    }
}

async function main(): Promise<void> {
    console.log(await selected());
    console.log(await transferred());
    try {
        await failed();
    } catch (error: Error) {
        console.log(error.message);
    }
    console.log(await overridden());
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
        "cleanup\n7\nloop cleanup\nloop cleanup\nloop cleanup\nloop cleanup\n2\nthrow cleanup\ncompletion failure\n9\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn channel_send_and_recv_deliver_a_value_in_fifo_order() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

async function main(): Promise<void> {
    const [tx, rx] = channel<u64>();
    await tx.send(1);
    await tx.send(2);
    const first = await rx.recv();
    const second = await rx.recv();
    console.log((first ?? 0) + (second ?? 0));
    rx.close();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "3\n");
}

#[test]
fn channel_recv_resolves_to_none_once_every_sender_is_dropped() {
    let project = AsyncProject::new(
        r#"import { Sender, channel } from "tyrus:task";
import { drop } from "tyrus:memory";

async function producer(tx: Sender<u64>): Promise<void> {
    await tx.send(7);
}

export async function main(): Promise<void> {
    const [tx, rx] = channel<u64>();
    // The call borrows `tx`; dropping the last sender is what ends the channel.
    await producer(tx);
    drop(tx);
    const first = await rx.recv();
    const second = await rx.recv();
    const drainedToNone = !second;
    console.log(first ?? 0, drainedToNone);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "7 true\n");
}

/// Passing a `Sender` to an `async` function borrows it. The frame used to
/// adopt the caller's reference and zero the caller's slot without the checker
/// knowing, so the caller's next `send` dereferenced null and crashed.
/// `Sender`, `Receiver` and `JoinHandle` are exported by `tyrus:task`. A binding
/// whose type is only inferred needs no import, but an annotation that names one
/// does, and the missing import is reported where the type is written, with the
/// module that exports it.
#[test]
fn a_task_handle_type_named_without_its_import_is_refused_with_the_module() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

async function produce(tx: Sender<u64>): Promise<void> {
    await tx.send(1);
}

export async function main(): Promise<void> {
    const [tx, rx] = channel<u64>(2);
    await produce(tx);
    const value = await rx.recv();
    console.log(value ?? 0);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "an annotation naming `Sender` without importing it must be refused"
    );
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    assert!(output.contains("E0412"), "expected E0412:\n{output}");
    assert!(
        output.contains("import { Sender } from \"tyrus:task\""),
        "expected the help to name the import:\n{output}"
    );
}

#[test]
fn channel_sender_stays_usable_after_an_async_call_borrows_it() {
    let project = AsyncProject::new(
        r#"import { Sender, channel } from "tyrus:task";
import { drop } from "tyrus:memory";

async function produce(tx: Sender<u64>): Promise<void> {
    await tx.send(1);
}

export async function main(): Promise<void> {
    const [tx, rx] = channel<u64>(4);
    await produce(tx);
    await tx.send(2);
    drop(tx);
    const first = await rx.recv();
    const second = await rx.recv();
    const third = await rx.recv();
    console.log(first ?? 0, second ?? 0, !third);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1 2 true\n");
}

#[test]
fn channel_try_send_and_try_recv_never_wait() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

export async function main(): Promise<void> {
    const [tx, rx] = channel<u64>(1);
    const accepted = tx.trySend(1);
    const rejected = tx.trySend(2);
    const drained = rx.tryRecv();
    const emptyNow = !rx.tryRecv();
    console.log(accepted, rejected, drained ?? 0, emptyNow);
    rx.close();
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
        "true false 1 true\n"
    );
}

#[test]
fn channel_carries_string_payloads() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

export async function main(): Promise<void> {
    const [tx, rx] = channel<string>();
    await tx.send("hello");
    const greeting = await rx.recv();
    console.log(greeting ?? "");
    rx.close();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "hello\n");
}

#[test]
fn channel_moving_a_sender_into_a_spawned_task_delivers_every_value() {
    let project = AsyncProject::new(
        r#"import { Sender, channel, spawn } from "tyrus:task";

async function producer(tx: Sender<u64>): Promise<void> {
    for (let i: u64 = 0; i < 5; i = i + 1) {
        await tx.send(i);
    }
}

export async function main(): Promise<void> {
    const [tx, rx] = channel<u64>();
    const handle = spawn(async (): Promise<void> => await producer(tx));
    let total: u64 = 0;
    for (let i = 0; i < 5; i = i + 1) {
        const value = await rx.recv();
        total = total + (value ?? 0);
    }
    await handle.join();
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\n");
    let ir = project.emitted_ir();
    assert!(
        ir.matches("async.closure.capture.sender.retain").count() >= 2,
        "each async frame that owns the sender must retain its own reference:\n{ir}"
    );
}

#[test]
fn channel_rejects_an_unsupported_payload_type() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

interface Point {
    x: i32;
    y: i32;
}

export async function main(): Promise<void> {
    const [tx, rx] = channel<Point>();
    rx.close();
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for an unsupported channel payload type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3036") || stderr.contains("E3036"),
        "expected `E3036` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn channel_rejects_shadowing_sender_and_receiver_as_local_names() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

interface Sender {
    value: i32;
}

export async function main(): Promise<void> {
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for shadowing the native `Sender` type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3040") || stderr.contains("E3040"),
        "expected `E3040` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn join_all_resolves_every_handle_in_order() {
    let project = AsyncProject::new(
        r#"import { spawn, joinAll } from "tyrus:task";

async function main(): Promise<void> {
    const first = spawn(async (): Promise<i32> => { return 10; });
    const second = spawn(async (): Promise<i32> => { return 20; });
    const third = spawn(async (): Promise<i32> => { return 30; });
    console.log("checkpoint joinAll: tasks spawned");
    const results: i32[] = await joinAll([first, second, third]);
    console.log("checkpoint joinAll: all tasks joined");
    console.log(results[0], results[1], results[2]);
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
        "program failed ({}):\nstdout:\n{}\nstderr:\n{}",
        process_status_description(&output.status),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "checkpoint joinAll: tasks spawned\ncheckpoint joinAll: all tasks joined\n10 20 30\n"
    );
}

#[test]
fn join_all_propagates_the_first_rejection() {
    let project = AsyncProject::new(
        r#"import { spawn, joinAll } from "tyrus:task";

async function failing(): Promise<i32> throws Error {
    const error: Error = { message: "task failed", code: "ERR_TASK_FAILED" };
    throw error;
}

async function main(): Promise<void> {
    const ok = spawn(async (): Promise<i32> => { return 1; });
    const bad = spawn(async (): Promise<i32> throws Error => await failing());
    try {
        const results: i32[] = await joinAll([ok, bad]);
        console.log("unexpected", results.length);
    } catch (error: Error) {
        console.log(error.message);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "task failed\n");
}

#[test]
fn race_resolves_to_the_first_settled_handle_and_cancels_the_rest() {
    let project = AsyncProject::new(
        r#"import { spawn, race, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const fast = spawn(async (): Promise<i32> => {
        await sleepAsync(10);
        return 1;
    });
    const slow = spawn(async (): Promise<i32> => {
        await sleepAsync(1500);
        return 2;
    });
    const winner: i32 = await race([fast, slow]);
    console.log(winner);
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
    let start = std::time::Instant::now();
    let output = project.run();
    let elapsed = start.elapsed();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "1\n");
    assert!(
        elapsed < std::time::Duration::from_millis(1000),
        "expected the process to exit shortly after the fast handle won, not wait for the slow one: {elapsed:?}"
    );
}

#[test]
fn race_rejects_reusing_a_handle_already_consumed_by_the_race() {
    let project = AsyncProject::new(
        r#"import { spawn, race } from "tyrus:task";

async function main(): Promise<void> {
    const first = spawn(async (): Promise<i32> => { return 1; });
    const second = spawn(async (): Promise<i32> => { return 2; });
    const winner: i32 = await race([first, second]);
    const again: i32 = await first.join();
    console.log(winner, again);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for reusing a handle already consumed by `race`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3033") || stderr.contains("E3033"),
        "expected `E3033` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn timeout_rejects_when_the_task_does_not_settle_in_time_and_resolves_otherwise() {
    let project = AsyncProject::new(
        r#"import { timeout, sleepAsync } from "tyrus:task";

async function slowTask(): Promise<i32> {
    await sleepAsync(300);
    return 42;
}

async function fastTask(): Promise<i32> {
    await sleepAsync(5);
    return 7;
}

async function main(): Promise<void> {
    try {
        const value: i32 = await timeout(50, slowTask());
        console.log("unexpected", value);
    } catch (error: Error) {
        console.log(error.message);
    }

    const value: i32 = await timeout(500, fastTask());
    console.log(value);
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
        "Timed out before the task completed\n7\n"
    );
}

#[test]
fn timeout_propagates_a_thrown_error_from_the_task() {
    let project = AsyncProject::new(
        r#"import { timeout } from "tyrus:task";

async function failingTask(): Promise<i32> throws Error {
    const error: Error = { message: "boom", code: "ERR_BOOM" };
    throw error;
}

async function main(): Promise<void> {
    try {
        const value: i32 = await timeout(500, failingTask());
        console.log("unexpected", value);
    } catch (error: Error) {
        console.log(error.message);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "boom\n");
}

#[test]
fn mutex_lock_and_try_lock_read_and_mutate_the_guarded_value() {
    let project = AsyncProject::new(
        r#"import { Mutex, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const counter = Mutex<u32>(0);
    const handle = spawn(async (): Promise<void> => {
        const guard = await counter.lock();
        guard.value += 1;
        guard.value += 41;
        console.log(guard.value);
        const maybeGuard = counter.tryLock();
        console.log(!maybeGuard);
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\ntrue\n");
}

#[test]
fn mutex_try_lock_fails_while_a_guard_from_lock_is_still_alive() {
    let project = AsyncProject::new(
        r#"import { Mutex, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const counter = Mutex<u32>(7);
    const handle = spawn(async (): Promise<void> => {
        const guard = await counter.lock();
        console.log(guard.value);
        const maybeGuard = counter.tryLock();
        if (maybeGuard) {
            console.log(maybeGuard.value);
        } else {
            console.log(-1);
        }
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "7\n-1\n");
}

#[test]
fn mutex_rejects_a_guard_held_across_an_await() {
    let project = AsyncProject::new(
        r#"import { Mutex, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const counter = Mutex<u32>(0);
    const guard = await counter.lock();
    await sleepAsync(1);
    guard.value += 1;
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for a `MutexGuard` held across an `await`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3034") || stderr.contains("E3034"),
        "expected `E3034` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn mutex_rejects_an_unsupported_payload_type() {
    let project = AsyncProject::new(
        r#"import { Mutex } from "tyrus:task";

interface Point {
    x: i32;
    y: i32;
}

export async function main(): Promise<void> {
    const guarded = Mutex<Point>({ x: 1, y: 2 });
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for an unsupported `Mutex` payload type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3037") || stderr.contains("E3037"),
        "expected `E3037` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn mutex_rejects_shadowing_mutex_and_mutex_guard_as_local_names() {
    let project = AsyncProject::new(
        r#"import { Mutex } from "tyrus:task";

interface MutexGuard {
    value: i32;
}

export async function main(): Promise<void> {
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for shadowing the native `MutexGuard` type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3044") || stderr.contains("E3044"),
        "expected `E3044` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rwlock_read_returns_the_guarded_value() {
    let project = AsyncProject::new(
        r#"import { RwLock, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<u32>(10);
    const handle = spawn(async (): Promise<void> => {
        const read = await config.read();
        console.log(read.value);
        const maybeWriter = config.tryWrite();
        console.log(!maybeWriter);
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "10\ntrue\n");
}

#[test]
fn rwlock_write_mutates_the_guarded_value_exclusively() {
    let project = AsyncProject::new(
        r#"import { RwLock, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<u32>(10);
    const handle = spawn(async (): Promise<void> => {
        const write = await config.write();
        write.value += 32;
        console.log(write.value);
        const maybeReader = config.tryRead();
        console.log(!maybeReader);
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "42\ntrue\n");
}

#[test]
fn rwlock_rejects_assigning_through_a_read_guard() {
    let project = AsyncProject::new(
        r#"import { RwLock, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<u32>(10);
    const handle = spawn(async (): Promise<void> => {
        const read = await config.read();
        read.value += 1;
        console.log(read.value);
    });
    await handle.join();
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for assigning through an `RwReadGuard`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E2542") || stderr.contains("E2542"),
        "expected `E2542` (readonly property) in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rwlock_rejects_a_guard_held_across_an_await() {
    let project = AsyncProject::new(
        r#"import { RwLock, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<u32>(0);
    const write = await config.write();
    await sleepAsync(1);
    write.value += 1;
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for an `RwWriteGuard` held across an `await`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3034") || stderr.contains("E3034"),
        "expected `E3034` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rwlock_rejects_an_unsupported_payload_type() {
    let project = AsyncProject::new(
        r#"import { RwLock } from "tyrus:task";

interface Point {
    x: i32;
    y: i32;
}

export async function main(): Promise<void> {
    const guarded = RwLock<Point>({ x: 1, y: 2 });
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for an unsupported `RwLock` payload type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3038") || stderr.contains("E3038"),
        "expected `E3038` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rwlock_rejects_shadowing_rwlock_and_guard_types_as_local_names() {
    let project = AsyncProject::new(
        r#"import { RwLock } from "tyrus:task";

interface RwWriteGuard {
    value: i32;
}

export async function main(): Promise<void> {
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for shadowing the native `RwWriteGuard` type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3046") || stderr.contains("E3046"),
        "expected `E3046` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn semaphore_acquire_limits_concurrent_access() {
    let project = AsyncProject::new(
        r#"import { Semaphore } from "tyrus:task";

async function main(): Promise<void> {
    const limit = Semaphore(1);
    const permit = await limit.acquire();
    const maybePermit = limit.tryAcquire();
    console.log(!maybePermit);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");
}

#[test]
fn semaphore_try_acquire_returns_a_real_present_optional() {
    let project = AsyncProject::new(
        r#"import { Semaphore } from "tyrus:task";

async function main(): Promise<void> {
    const limit = Semaphore(1);
    const maybePermit = limit.tryAcquire();
    if (maybePermit) {
        console.log("acquired");
    } else {
        console.log("missing");
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "acquired\n");
}

#[test]
fn semaphore_allows_up_to_its_own_permit_count_at_once() {
    let project = AsyncProject::new(
        r#"import { Semaphore } from "tyrus:task";

async function main(): Promise<void> {
    const limit = Semaphore(2);
    const first = await limit.acquire();
    const second = await limit.acquire();
    const maybeThird = limit.tryAcquire();
    console.log(!maybeThird);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "true\n");
}

#[test]
fn semaphore_permit_is_allowed_to_stay_alive_across_an_await() {
    let project = AsyncProject::new(
        r#"import { Semaphore, sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const limit = Semaphore(1);
    const permit = await limit.acquire();
    await sleepAsync(1);
    console.log("survived");
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "survived\n");
}

#[test]
fn semaphore_rejects_shadowing_semaphore_and_permit_as_local_names() {
    let project = AsyncProject::new(
        r#"import { Semaphore } from "tyrus:task";

interface SemaphorePermit {
    id: i32;
}

export async function main(): Promise<void> {
    console.log("unreachable");
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for shadowing the native `SemaphorePermit` type"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("E3047") || stderr.contains("E3047"),
        "expected `E3047` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Program source used by every worker-thread test: it has to `await`, so the
/// executor is really built with the configured count.
const SLEEPING_MAIN: &str = r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    await sleepAsync(1);
    console.log("done");
}
"#;

#[test]
fn runtime_bakes_the_configured_worker_thread_count_into_the_program() {
    let project = AsyncProject::with_runtime_config(
        SLEEPING_MAIN,
        "\n  \"runtime\": {\n    \"workerThreads\": 3\n  },",
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
            .contains("call void @tyrus_runtime_configure_worker_threads(i64 3)"),
        "the configured worker-thread count was not emitted into `main`"
    );

    let output = project.run();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "done\n");
}

#[test]
fn runtime_leaves_the_worker_thread_count_unset_when_the_project_does_not_configure_one() {
    let project = AsyncProject::new(SLEEPING_MAIN);
    let build = project.build();
    assert!(build.status.success());
    assert!(
        !project
            .emitted_ir()
            .contains("tyrus_runtime_configure_worker_threads"),
        "an unconfigured project must not bake a worker-thread count in"
    );
}

#[test]
fn runtime_rejects_a_worker_thread_count_below_one() {
    let project = AsyncProject::with_runtime_config(
        SLEEPING_MAIN,
        "\n  \"runtime\": {\n    \"workerThreads\": 0\n  },",
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "a zero worker count must not build"
    );
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("`runtime.workerThreads` must be at least 1"),
        "got: {stderr}"
    );
}

#[test]
fn runtime_warns_when_the_configured_worker_threads_exceed_the_detected_capacity() {
    let threads = oversubscribed_worker_threads();
    let project = AsyncProject::with_runtime_config(
        SLEEPING_MAIN,
        &format!("\n  \"runtime\": {{\n    \"workerThreads\": {threads}\n  }},"),
    );
    assert!(project.build().status.success());

    let output = project.run();
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("runtime.workerThreads") && stderr.contains("logical CPUs were detected"),
        "got: {stderr}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "done\n",
        "the warning must not reach stdout"
    );
}

#[test]
fn runtime_lets_the_environment_variable_override_the_configured_worker_threads() {
    let project = AsyncProject::with_runtime_config(
        SLEEPING_MAIN,
        "\n  \"runtime\": {\n    \"workerThreads\": 2\n  },",
    );
    assert!(project.build().status.success());

    let threads = oversubscribed_worker_threads().to_string();
    let output = project.run_with_env(&[("TYRUS_WORKER_THREADS", threads.as_str())]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("TYRUS_WORKER_THREADS requests"),
        "the environment variable must win over `runtime.workerThreads`, got: {stderr}"
    );
    assert!(
        !stderr.contains("runtime.workerThreads requests"),
        "got: {stderr}"
    );
}

#[test]
fn runtime_ignores_a_worker_threads_environment_variable_that_is_not_a_positive_integer() {
    let project = AsyncProject::new(SLEEPING_MAIN);
    assert!(project.build().status.success());

    for value in ["0", "many"] {
        let output = project.run_with_env(&[("TYRUS_WORKER_THREADS", value)]);
        assert!(
            output.status.success(),
            "an invalid value must not be fatal"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("is not a positive integer"),
            "got: {stderr}"
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "done\n");
    }
}

#[test]
fn runtime_reports_tasks_still_pending_when_main_finishes() {
    let project = AsyncProject::new(
        r#"import { sleepAsync, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const abandoned = spawn(async (): Promise<void> => {
        await sleepAsync(60000);
        console.log("unreachable");
    });
    console.log("done");
}
"#,
    );
    assert!(project.build().status.success());

    let quiet = project.run();
    assert!(quiet.status.success());
    assert_eq!(
        String::from_utf8_lossy(&quiet.stdout),
        "done\n",
        "a cancelled task must not have run to completion"
    );
    assert_eq!(
        String::from_utf8_lossy(&quiet.stderr),
        "",
        "the report is opt-in: `race` and `timeout` end with pending tasks by design"
    );

    let output = project.run_with_env(&[("TYRUS_REPORT_PENDING_TASKS", "1")]);
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("1 asynchronous task still pending when `main` finished"),
        "got: {stderr}"
    );
}

#[test]
fn runtime_stays_silent_when_every_task_completed_before_main_finished() {
    let project = AsyncProject::new(
        r#"import { sleepAsync, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        await sleepAsync(1);
        return 7;
    });
    const value = await handle.join();
    console.log(value);
}
"#,
    );
    assert!(project.build().status.success());

    let output = project.run_with_env(&[("TYRUS_REPORT_PENDING_TASKS", "1")]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "a joined task must not be reported as abandoned, even when asked for"
    );
}

#[test]
fn accepts_joining_a_handle_inside_an_enclosing_expression() {
    // Regression: an `await` nested in another expression is checked twice,
    // once while the enclosing expression is inferred and once while it is
    // checked, and the second visit used to report the very same `.join()`
    // call as a second consumption of the handle.
    let project = AsyncProject::new(
        r#"import { spawn } from "tyrus:task";

async function main(): Promise<void> {
    const handle = spawn(async (): Promise<i32> => {
        return 20;
    });
    const other = spawn(async (): Promise<i32> => {
        return 22;
    });
    console.log(await handle.join());
    const total: i32[] = [await other.join()];
    console.log(total[0]);
}
"#,
    );
    let build = project.build();
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        build.status.success(),
        "a single `.join()` inside an enclosing expression must build:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let output = project.run();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "20\n22\n");
}

#[test]
fn keeps_an_array_local_alive_across_a_suspension() {
    // Regression: the contained-array optimization reserved the buffer on the
    // native stack, which an `async` function does not keep across a
    // suspension, its locals live in a heap frame, so the next poll read the
    // array back through a pointer into a stack frame that was already gone
    // and the program died on SIGSEGV.
    let project = AsyncProject::new(
        r#"import { sleepAsync, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const numbers: i32[] = [1, 2, 3];
    await sleepAsync(1);
    console.log(numbers[0] + numbers[1] + numbers[2]);

    const handle = spawn(async (): Promise<i32> => {
        const inner: i32[] = [10, 20];
        await sleepAsync(1);
        return inner[0] + inner[1];
    });
    const total = await handle.join();
    console.log(total);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );

    let output = project.run();
    assert!(
        output.status.success(),
        "an array local must survive a suspension, got status {:?}",
        output.status
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "6\n30\n");
}

fn assert_build_error(source: &str, code: &str) {
    let project = AsyncProject::new(source);
    let build = project.build();
    assert!(
        !build.status.success(),
        "expected a compile error for {code}"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains(code) || stderr.contains(code),
        "expected `{code}` in build output:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn rejects_a_channel_with_no_capacity() {
    assert_build_error(
        r#"import { channel } from "tyrus:task";

async function main(): Promise<void> {
    const [tx, rx] = channel<i32>(0);
}
"#,
        "E3035",
    );
}

#[test]
fn rejects_a_semaphore_with_no_permits() {
    assert_build_error(
        r#"import { Semaphore } from "tyrus:task";

async function main(): Promise<void> {
    const limit = Semaphore(0);
}
"#,
        "E3035",
    );
}

#[test]
fn accepts_a_channel_and_a_semaphore_with_room_for_one() {
    let project = AsyncProject::new(
        r#"import { channel, Semaphore } from "tyrus:task";

async function main(): Promise<void> {
    const [tx, rx] = channel<i32>(1);
    await tx.send(7);
    const value = await rx.recv();
    console.log(value ?? 0);

    const limit = Semaphore(1);
    const permit = await limit.acquire();
    console.log("acquired");
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "7\nacquired\n");
}

/// A `Promise<T>` is consumed by `await` exactly as a `JoinHandle<T>` is by
/// `.join()`: the runtime holds one reference, and `await` takes and releases
/// it. Before this rule existed the second `await` compiled and the program
/// died on SIGSEGV, reading a promise the first one had already freed.
#[test]
fn rejects_awaiting_the_same_promise_binding_twice() {
    assert_build_error(
        r#"async function value(): Promise<i32> {
    return 7;
}

async function main(): Promise<void> {
    const pending = value();
    const first = await pending;
    const second = await pending;
    console.log(first + second);
}
"#,
        "E3006",
    );
}

#[test]
fn accepts_awaiting_each_promise_binding_once() {
    let project = AsyncProject::new(
        r#"async function value(): Promise<i32> {
    return 7;
}

async function main(): Promise<void> {
    const first = value();
    const second = value();
    console.log(await first);
    const total = await second;
    console.log(total);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n7\n");
}

/// A borrow owns nothing, so an `async` frame can hold one: it stores the bytes
/// and releases none of them. Codegen used to refuse outright, with "async
/// frame storage for non-Copy local `b` of type `&string` is not implemented
/// yet", because the classifier looks through the `&` and then asks an
/// ownership question a borrow has no answer to.
#[test]
fn an_async_function_accepts_a_borrow_that_does_not_cross_an_await() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    await sleepAsync(1);
    const owner: string = "Ada Lovelace";
    const borrowed: &string = &owner;
    console.log(borrowed);
    console.log(borrowed.length);

    let counter: i32 = 41;
    const slot: &mut i32 = &mut counter;
    console.log(slot);
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
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Ada Lovelace\n12\n41\n"
    );
}

/// The exemption above is only sound because a borrow can never be live at a
/// suspension point, so the frame slot is written and read inside one poll.
/// That rule has to keep firing.
#[test]
fn an_async_function_still_rejects_a_borrow_held_across_an_await() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const owner: string = "Ada";
    const borrowed: &string = &owner;
    await sleepAsync(1);
    console.log(borrowed);
}
"#,
    );
    let build = project.build();
    assert!(
        !build.status.success(),
        "a borrow must not be allowed to cross an `await`"
    );
    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stdout.contains("crosses `await`") || stderr.contains("crosses `await`"),
        "expected the borrow-across-await error:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

/// Inside an `async` function nothing registers an owned allocation: the frame
/// reconstructs every release from the *shape* of the initializer. A number
/// rendering itself to a string matched no shape, so its buffer was never freed,
/// and a `const s: string = n.toString();` local was refused outright, since
/// the same predicate decides whether the frame can hold it.
#[test]
fn an_async_function_owns_a_string_rendered_from_a_number() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    await sleepAsync(1);
    let round: i32 = 0;
    let total: usize = 0;
    while (round < 3) {
        const joined: string = "round" + round.toString();
        const bare: string = round.toString();
        const ratio: string = (1.5).toString();
        total += joined.length + bare.length + ratio.length;
        round += 1;
    }
    console.log(total);
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
    assert!(output.status.success());
    // "round0"/"round1"/"round2" is 6, the bare digit 1, "1.5" 3, ten per round.
    assert_eq!(String::from_utf8_lossy(&output.stdout), "30\n");
}

/// Keep the file result intact across suspension, then consume its string
/// payload in the match arm. The array's joined string also survives an await.
#[test]
fn an_async_function_owns_read_file_result_and_array_join_string() {
    let project = AsyncProject::new(
        r#"import { FsError, readFileSync, writeFileSync } from "tyrus:fs";
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    match (writeFileSync("target/async-owned-string.txt", "hello")) {
        Ok() => {}
        Err(error) => { console.log("fs error", error.code); return; }
    }
    const readResult: Result<string, FsError> = readFileSync("target/async-owned-string.txt");
    await sleepAsync(1);
    match (readResult) {
        Ok(text) => { console.log(text, text.length); }
        Err(error) => { console.log("fs error", error.code); return; }
    }
    let parts: string[] = [];
    parts.push("a");
    parts.push("b");
    const joined: string = parts.join("-");
    await sleepAsync(1);
    console.log(joined);
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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello 5\na-b\n");
}

/// The boundary of the rule above: only a number rendering itself allocates.
/// A `string` receiver returns an alias of its own buffer and a `boolean` a
/// static constant, so neither may ever be claimed by the frame, claiming one
/// would free a buffer the binding does not own. The frame answers that by
/// storing a clone of the value, which it then owns outright, and never the
/// alias itself: the source keeps its buffer and each side releases its own.
#[test]
fn an_async_frame_stores_a_clone_of_a_string_local_it_does_not_own() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    await sleepAsync(1);
    const source: string = `text ${1}`;
    const flag: boolean = true;
    const same: string = source.toString();
    const shown: string = flag.toString();
    await sleepAsync(1);
    console.log(same, shown, source);
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
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "text 1 true text 1\n"
    );
}

/// A `tryLock`/`tryRead`/`tryWrite` result stored in an `async` frame local
/// used to segfault, deterministically, in an ordinary debug build.
///
/// The three runtime entry points hand back the acquired guard's own pointer,
/// or null, described at the call sites as "exactly the `Optional<Guard>`
/// representation already". It is not one: a `T?` here is a heap box holding
/// the payload, which is what every consumer of an optional dereferences.
/// Cloning the bare pointer into an async frame therefore read the guard
/// struct's *first field* (the lock handle) and stored that as the guard, so
/// the frame's drop glue later unlocked a `TyrusRwLock` as a
/// `TyrusRwLockWriteGuard` and dropped a permit that was never there.
///
/// Capturing the lock into a `spawn` is what puts the guard in a frame, which
/// is why this shape crashed and a plain `const g = lock.tryWrite();` at top
/// level did not. Both locks are covered because both share the entry points.
#[test]
fn a_try_lock_guard_stored_in_an_async_frame_is_a_real_optional() {
    let project = AsyncProject::new(
        r#"import { RwLock, Mutex, spawn } from "tyrus:task";

async function main(): Promise<void> {
    const config = RwLock<u32>(10);
    const counter = Mutex<u32>(7);

    const handle = spawn(async (): Promise<void> => {
        const maybeWriter = config.tryWrite();
        console.log(!maybeWriter);
        const maybeGuard = counter.tryLock();
        console.log(!maybeGuard);
    });
    await handle.join();
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "false\nfalse\n");
    let ir = project.emitted_ir();
    assert!(
        !ir.contains("async.optional.clone"),
        "fresh optional guards must move into the async frame instead of allocating a second box:\n{ir}"
    );
}

/// Channel endpoints declared inside a loop are released once per iteration,
/// and the values they carry stay valid until the code is done with them.
///
/// The frame has one slot per name and one teardown, so a loop redeclaring
/// `[tx, rx]` released only the last iteration's endpoints; clearing the slot
/// before each store is what fixed it. The risk that introduces is the
/// opposite one (releasing a value the next statement still reads) so this
/// asserts the received message, not just that the program exits.
#[test]
fn channel_endpoints_redeclared_in_a_loop_stay_valid_each_iteration() {
    let project = AsyncProject::new(
        r#"import { channel } from "tyrus:task";

export async function main(): Promise<void> {
    let total: u64 = 0;
    for (let i: i32 = 0; i < 3; i += 1) {
        const [tx, rx] = channel<u64>(1);
        const accepted = tx.trySend(7);
        const drained = rx.tryRecv();
        total += (drained ?? 0) + (accepted ? 1 : 0);
        rx.close();
    }
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "24\n");
}

#[test]
fn an_async_loop_rebinding_an_owned_array_frees_it_exactly_once() {
    // The other half of re-enabling temporary tracking inside async functions.
    // `transfer_owned_native_result` located the pending entry of a value being
    // bound to a name and then, inside an async function, returned without
    // removing it, harmless only while nothing was ever recorded. With
    // recording on for `await`-free statements it became a use-after-free: the
    // heap frame's drop glue released `numbers` at scope exit and the
    // statement-end sweep released it again at the end of the declaration that
    // introduced it, every iteration.
    //
    // The array is redeclared each iteration so a stale free is read back by
    // the next one rather than only at exit.
    let project = AsyncProject::new(
        r#"export async function main(): Promise<void> {
    let total: usize = 0;
    for (let i: i32 = 0; i < 4; i += 1) {
        const numbers: i32[] = [1, 2, 3];
        total += numbers.concat([4]).length + numbers.length;
    }
    console.log(total);
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "28\n");
}

#[test]
fn zeroize_in_an_async_function_returns_memory_error_before_a_later_await() {
    let project = AsyncProject::new(
        r#"import { MemoryError, zeroize } from "tyrus:memory";

export async function main(): Promise<void> {
    let token: string = "async-token".slice(0);
    let bytes: u8[] = [1, 2, 3];
    match (zeroize(token)) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
    }
    match (zeroize(bytes)) {
        Ok() => { console.log("erased"); }
        Err(error) => { console.log(error.code); }
    }

    await Promise.resolve<void>();

    const literal = "static-token";
    match (zeroize(literal)) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
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
        "erased\nERR_MEMORY_NOT_OWNED\n"
    );
    let ir = project.emitted_ir();
    assert!(ir.contains("tyrus_memory_zeroize_string"), "{ir}");
    assert!(ir.contains("tyrus_memory_zeroize_u8_array"), "{ir}");
}

#[test]
fn drop_in_an_async_function_releases_owned_values_before_a_later_await() {
    let project = AsyncProject::new(
        r#"import { drop } from "tyrus:memory";

export async function main(): Promise<void> {
    let text: string = "async drop".slice(0);
    let values: i32[] = [1, 2, 3];
    drop(text);
    drop(values);
    await Promise.resolve<void>();
    console.log("dropped");
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
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "dropped\n");
    let ir = project.emitted_ir();
    assert!(ir.contains("tyrus_string_release_buffer"), "{ir}");
    let heap_free_call = if cfg!(windows) {
        "call void @tyrus_runtime_heap_free"
    } else {
        "call void @free"
    };
    assert!(
        ir.matches(heap_free_call).count() >= 3,
        "the explicit array drop must release its buffer before the frame teardown: {ir}"
    );
}

/// An `async` arrow nested in a synchronous closure may bind the result of an
/// `await`. The code generator's pre-flight validation used to apply the
/// synchronous rules to the nested arrow's body and reject
/// `const value = await f();` with "await codegen is not implemented yet",
/// which broke every `async` test written inside a `describe`.
#[test]
fn async_arrow_nested_in_a_sync_closure_binds_an_await() {
    let project = AsyncProject::new(
        r#"async function fetchCount(): Promise<i32> {
    return 3;
}

const makeTask = (): void => {
    const task = async (): Promise<void> => {
        const count = await fetchCount();
        console.log(count);
    };
    console.log("task defined");
};

async function main(): Promise<void> {
    makeTask();
    const direct = async (): Promise<i32> => {
        const count = await fetchCount();
        return count + 1;
    };
    console.log(await direct());
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "task defined\n4\n");
}

/// A `throw` from the synchronous part of a suspending `try` reaches the
/// handler written around it.
///
/// This used to walk past the `catch` and become an unhandled rejection: the
/// handler was in the source, was read as covering the call, and never ran.
/// What the lowering was missing is a jump target for the stretches that run
/// synchronously, which are exactly the stretches a `longjmp` can reach,
/// because they run on the poll call that installed it.
#[test]
fn a_synchronous_throw_before_the_await_reaches_the_catch() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

function boom(): i32 throws string {
    throw "boom";
}

async function main(): Promise<void> {
    try {
        const value: i32 = boom();
        await sleepAsync(1);
        console.log(value);
    } catch (error: string) {
        console.log("caught", error);
    }
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "caught boom\nafter\n");
}

/// The stretch after the suspension resumes on a later poll call, and gets a
/// jump target of its own there.
#[test]
fn a_synchronous_throw_after_the_await_reaches_the_catch() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

function boom(): i32 throws string {
    throw "late";
}

async function main(): Promise<void> {
    try {
        await sleepAsync(1);
        const value: i32 = boom();
        console.log(value);
    } catch (error: string) {
        console.log("caught", error);
    }
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "caught late\nafter\n");
}

/// Every path through a suspending `try`: no throw, a throw before the
/// suspension, and a throw after it, each running `finally` exactly once.
#[test]
fn a_suspending_try_runs_finally_on_every_path() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

function step(fail: boolean, tag: string): i32 throws string {
    if (fail) {
        throw tag;
    }
    return 1;
}

async function run(early: boolean, late: boolean): Promise<void> {
    try {
        const before: i32 = step(early, "early");
        await sleepAsync(1);
        const after: i32 = step(late, "late");
        console.log("ok", before + after);
    } catch (error: string) {
        console.log("caught", error);
    } finally {
        console.log("finally");
    }
}

async function main(): Promise<void> {
    await run(false, false);
    await run(true, false);
    await run(false, true);
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
    assert_eq!(
        String::from_utf8_lossy(&run.stdout),
        "ok 2\nfinally\ncaught early\nfinally\ncaught late\nfinally\n"
    );
}

/// A handler that returns still ends the function with the caught value's
/// answer, and the value it caught is released rather than carried out.
#[test]
fn a_catch_reached_by_a_synchronous_throw_can_return() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

function boom(): i32 throws string {
    throw "failure";
}

async function run(): Promise<i32> {
    try {
        const value: i32 = boom();
        await sleepAsync(1);
        return value;
    } catch (error: string) {
        const size: i32 = error.length as i32;
        return size;
    }
}

async function main(): Promise<void> {
    const answer = await run();
    console.log(answer);
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "7\n");
}

/// A `catch` that awaits resumes on a later poll call, which no jump from the
/// throwing one can reach, so the checker used to refuse the combination. The
/// guarded poll hands the error to that `catch` as a rejection instead.
#[test]
fn a_suspending_catch_receives_a_synchronous_throw() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

function boom(): i32 throws string {
    throw "boom";
}

async function main(): Promise<void> {
    try {
        const value: i32 = boom();
        await sleepAsync(1);
        console.log(value);
    } catch (error: string) {
        await sleepAsync(1);
        console.log("caught", error);
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
    assert!(run.status.success(), "{run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stdout), "caught boom\n");
}

/// A native handle survives a suspension with a `?` around it.
///
/// The frame recognized `HttpServer` and not `HttpServer?`, though both are one
/// pointer and absence is the null one. Two more gaps sat behind that: the
/// asynchronous `if` read its condition without the helper that makes an
/// optional truthy when present, and it recorded no narrowing, so
/// `server.close()` inside the branch still saw the declared optional.
#[test]
fn an_optional_native_handle_survives_a_suspension() {
    if loopback_bind_is_denied() {
        return;
    }
    let project = AsyncProject::new(
        r#"import { createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions, TextBody } from "tyrus:http";
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("ok");
        },
    });
    let server: HttpServer?;
    const started = app.listen(0, { host: "127.0.0.1" }) onfail {
        console.log("start failed", started.code);
        return;
    };
    server = started;
    await sleepAsync(1);
    if (server) {
        await server.close();
        console.log("closed");
    } else {
        console.log("no server");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "closed\n");
}

/// A route with a JSON response schema can be registered from an async function.
///
/// The handler is a plain function even when the code registering the route is
/// an async one, and it was being emitted while the enclosing frame's promise
/// was still visible. A `return` in the handler then settled the *caller's*
/// result: `tyrus_promise_fulfill` on an argument belonging to another
/// function, which LLVM rejects, so the program did not compile at all.
#[test]
fn a_json_route_registered_from_an_async_function_compiles() {
    if loopback_bind_is_denied() {
        return;
    }
    let project = AsyncProject::new(
        r#"import { createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions, TextBody } from "tyrus:http";
import { sleepAsync } from "tyrus:task";

interface Count {
    readonly size: i32;
}

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/note", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("note");
        },
    });
    app.get("/count", {
        response: { 200: Count },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send({ size: 7 });
        },
    });
    try {
        const server = app.listen(0, { host: "127.0.0.1" }) onfail {
            console.log("start failed", server.code);
            return;
        };
        await sleepAsync(1);
        await server.close();
        console.log("served");
    } catch (error: HttpError) {
        console.log("start failed", error.code);
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "served\n");
}

/// A router created in an async function is released when its frame is dropped.
///
/// The frame stored the handle in an owned slot and its drop glue reached the
/// arm that releases nothing, on the grounds that an opaque handle belongs to
/// the runtime and the program gives it up explicitly. A router has no such
/// call, so nothing freed it. Asserted on the emitted drop function because a
/// leak is invisible to a program's output; `tests/leaks/clean` measures the
/// bytes.
#[test]
fn a_router_created_in_an_async_function_is_released() {
    let project = AsyncProject::new(
        r#"import { createRouter, HttpReply, HttpRouter, HttpStatus, TextBody } from "tyrus:http";
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("ok");
        },
    });
    await sleepAsync(1);
    console.log("done");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "done\n");
    let ir = project.emitted_ir();
    let drop_function = async_drop_function(&ir, "main");
    assert!(
        drop_function.contains("tyrus_http_router_free"),
        "the frame that owns a router must release it:\n{drop_function}"
    );
}

/// Moving a router to another local leaves exactly one name releasing it.
///
/// A frame slot takes a counted handle by retaining it, which lets the source
/// keep a release of its own; a move-only handle gets no such reference, so
/// both slots name one allocation. With the ownership bit left set on both, the
/// frame drop released the same router twice. The language already calls this a
/// move -- naming `app` afterwards does not compile -- so the source gives its
/// ownership up at the move.
#[test]
fn a_router_moved_to_another_local_is_released_once() {
    let project = AsyncProject::new(
        r#"import { createRouter, HttpReply, HttpRouter, HttpStatus, TextBody } from "tyrus:http";
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("ok");
        },
    });
    const owner: HttpRouter = app;
    await sleepAsync(1);
    owner.get("/second", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("second");
        },
    });
    console.log("done");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "done\n");
    let ir = project.emitted_ir();
    let poll = async_poll_function(&ir, "main");
    assert!(
        poll.contains("store i1 false, ptr %async.frame.local.app.initialized"),
        "the source of a move must give its ownership up:\n{poll}"
    );
}

/// Starting a server hands back a reference the program has to end up owning.
///
/// A listen result widened into `HttpServer?` retains the handle in the async
/// frame and releases the temporary optional box without dropping the handle
/// still owned by the source binding.
#[test]
fn a_server_widened_into_an_optional_keeps_one_reference() {
    if loopback_bind_is_denied() {
        return;
    }
    let project = AsyncProject::new(
        r#"import { createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus, ServerOptions, TextBody } from "tyrus:http";
import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    const app: HttpRouter = createRouter();
    app.get("/", {
        response: { 200: TextBody },
        handler: (request, reply): void => {
            reply.status(HttpStatus.Ok).send("ok");
        },
    });
    let server: HttpServer?;
    const started = app.listen(0, { host: "127.0.0.1" }) onfail {
        console.log("listen failed", started.code);
        return;
    };
    server = started;
    await sleepAsync(1);
    if (server) {
        await server.close();
        console.log("closed");
    } else {
        console.log("no server");
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
    let ir = project.emitted_ir();
    let poll = async_poll_function(&ir, "main");
    assert!(
        poll.contains(
            "async.closure.capture.http_server.retain = call ptr @tyrus_http_server_retain"
        ),
        "widening the handle must retain a reference for the optional frame slot:\n{poll}"
    );
    assert!(
        poll.contains(&format!(
            "call void @{}(ptr %optional_storage)",
            if cfg!(windows) {
                "tyrus_runtime_heap_free"
            } else {
                "free"
            }
        )),
        "widening the handle must release the temporary optional box without freeing the retained handle:\n{poll}"
    );
    let run = project.run();
    assert!(
        run.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout), "closed\n");
}

/// An optional bound across a suspension narrows inside a branch that awaits.
#[test]
fn an_await_inside_a_narrowed_branch_sees_the_narrowed_type() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function twice(value: i32): Promise<i32> {
    await sleepAsync(1);
    return value * 2;
}

async function main(): Promise<void> {
    const maybe: string? = "ready";
    if (maybe) {
        const size: i32 = maybe.length as i32;
        const doubled = await twice(size);
        console.log(doubled);
    } else {
        console.log("absent");
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
    assert_eq!(String::from_utf8_lossy(&run.stdout), "10\n");
}

/// A `.copy()` result can be held across an `await`.
///
/// `copy()` lowers to the same deep clone the frame itself uses to take
/// ownership, so its result is always a buffer of its own: the one thing the
/// frame's ownership lists exist to establish. It was missing from all three of
/// them anyway, which made this the sharpest edge of the gap: `E3081` refuses to
/// read a non-Copy field out of a value and points the reader at `.copy()`, and
/// the expression it recommends could not then survive a suspension.
///
/// All three shapes at once, because each is a separate list: the plain buffer
/// of a `u8[]`, a `string`, and a `string[]`, whose elements are cloned too.
#[test]
fn a_copy_is_owned_enough_to_cross_an_await() {
    let project = AsyncProject::new(
        r#"interface Bag {
    readonly bytes: u8[];
    readonly label: string;
    readonly names: string[];
}

async function seven(): Promise<i32> {
    return 7;
}

async function inspect(bag: Bag): Promise<void> {
    const bytes: u8[] = bag.bytes.copy();
    const label: string = bag.label.copy();
    const names: string[] = bag.names.copy();
    const value: i32 = await seven();
    console.log(bytes.length, label, names[1], value);
}

async function main(): Promise<void> {
    const bag: Bag = { bytes: [1, 2, 3], label: "hello", names: ["alpha", "beta"] };
    await inspect(bag);
    // The source is read again afterwards: a clone that aliased it would have
    // been freed by the frame above, and this would read released memory.
    await inspect(bag);
    console.log(bag.names[0]);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "3 hello beta 7\n3 hello beta 7\nalpha\n"
    );
}

/// An `async` arrow that stays an expression runs its `await`.
///
/// A named `const f = async () => { ... }` becomes a declaration and was always
/// fine. One nested inside an object literal is not, and its body was being
/// read by the walk that checks synchronous code, which refuses `await` as
/// unimplemented codegen. The codegen was there all along; only the gate was
/// wrong, so this test asserts the value the `await` produced rather than
/// merely that the build succeeded.
///
/// The function holding the literal is deliberately synchronous. Inside an
/// `async` one the object literal is never walked by that check at all, so the
/// same nesting there proves nothing: this is how the HTTP route that found it
/// was written, with an inline handler on a router built by a plain function.
#[test]
fn an_async_arrow_nested_in_an_object_literal_can_await() {
    let project = AsyncProject::new(
        r#"interface Work {
    readonly run: () => Promise<i32>;
}

async function seven(): Promise<i32> {
    return 7;
}

function build(): Work {
    return {
        run: async (): Promise<i32> => {
            const value: i32 = await seven();
            return value + 1;
        },
    };
}

async function main(): Promise<void> {
    const work: Work = build();
    const result: i32 = await work.run();
    console.log(result);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "8\n");
}

/// A synchronous function still refuses `await`.
///
/// The fix above is a branch, not a removal: the walk that refuses `await` is
/// the right one for a body that is not `async`, and an arrow without the
/// keyword must keep reaching it.
#[test]
fn a_synchronous_arrow_still_refuses_an_await() {
    let project = AsyncProject::new(
        r#"async function seven(): Promise<i32> {
    return 7;
}

function main(): void {
    const run = (): void => {
        const value = await seven();
        console.log(value);
    };
    run();
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("`await` can only be used inside an async function"),
        "unexpected diagnostic: {stderr}"
    );
}

/// A socket address read into a local can cross an `await`.
///
/// `localAddress` and `peerAddress` build the text from the address and hand
/// back a fresh allocation every time, so a frame holding one owns it. The
/// proof needed a list of its own: the existing one is about methods *on a
/// string* and asks for a `Str` receiver, which a `TcpListener` never is, so
/// putting the name there would have changed nothing.
///
/// Before this, the only way to use an address inside an `async` function was
/// to never bind it, which is not a workaround anyone would find on purpose.
#[test]
fn a_socket_address_is_owned_enough_to_cross_an_await() {
    if loopback_bind_is_denied() {
        return;
    }
    let project = AsyncProject::new(
        r#"import { NetError, tcpListen, TcpListener } from "tyrus:net";

async function seven(): Promise<i32> {
    return 7;
}

async function inspect(): Promise<Result<void, NetError>> {
    const listener: TcpListener = await tcpListen("127.0.0.1:0")?;
    const address: string = listener.localAddress();
    const value: i32 = await seven();
    console.log(address.startsWith("127.0.0.1:"), value);
    listener.close();
    return Ok();
}

async function main(): Promise<void> {
    match (await inspect()) {
        Ok() => {}
        Err(error) => { console.log(error.code); }
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
    let ir = project.emitted_ir();
    let inspect_poll = async_poll_function(&ir, "__tyrus_module_0_inspect");
    assert!(
        inspect_poll.contains("zeroinitializer, ptr %\"async.frame.local.await#0\""),
        "`?` must clear the consumed awaited `Result` payload before frame cleanup:\n{inspect_poll}"
    );
    let result_success = inspect_poll
        .split("result.success:")
        .nth(1)
        .and_then(|block| block.split("result.error:").next())
        .expect("the awaited Result success block must be emitted");
    assert!(
        result_success.contains("store ptr %result.success.value, ptr %async.frame.local.listener"),
        "the listener handle must move directly from the Result into its async local:\n{result_success}"
    );
    let address_symbol = if cfg!(windows) {
        "@__tyrus_windows_string_abi_tyrus_net_tcp_listener_local_address"
    } else {
        "@tyrus_net_tcp_listener_local_address"
    };
    let address_call = inspect_poll
        .find(address_symbol)
        .expect("the listener address call must be emitted");
    assert!(
        !result_success.contains("@tyrus_net_tcp_listener_free"),
        "the consumed Result payload must not be freed after it moves into the listener local:\n{result_success}"
    );
    assert!(address_call > inspect_poll.find("result.success:").unwrap());
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "true 7\n");
}

/// The values an `async` function may hold across a suspension come from one
/// list, not three.
///
/// Which array methods return a freshly allocated array was written down twice:
/// once for synchronous code and once, smaller, for async frames. The second
/// copy had drifted, so `unique`, `scan`, `map` and `filter` on a `string[]`,
/// `toSorted`, and every other name the smaller list had never gained could not
/// be held across an `await` at all, while the synchronous side had always
/// released them correctly. The async lists now ask the shared one.
///
/// The deep cleanup this authorizes for a `string[]` is correct because every
/// one of these builds its result through the push that takes a reference of
/// its own per element: the array owns its strings whether they came from a
/// literal, from another array, or from a closure that returned its input
/// unchanged. Both of those `map` shapes are here for that reason.
#[test]
fn every_allocating_method_result_can_cross_an_await() {
    let project = AsyncProject::new(
        r#"interface Holder {
    readonly label: string;
    readonly size: i32;
}

async function seven(): Promise<i32> {
    return 7;
}

async function probe(): Promise<void> {
    const text: string = "abcdef";
    const nums: i32[] = [3, 1, 3, 2];
    const words: string[] = ["beta", "alpha", "beta"];
    const holder: Holder = { label: "x", size: 1 };

    const letter: string = text.charAt(1);
    const rendered: string = holder.toJson();
    const distinct: i32[] = nums.unique();
    const running: i32[] = nums.scan(0, (acc: i32, n: i32): i32 => acc + n);
    const sorted: string[] = words.toSorted();
    const kept: string[] = words.filter((w: string): boolean => w.length > 4);
    const shouted: string[] = words.map((w: string): string => w.toUpperCase());
    const same: string[] = words.map((w: string): string => w);

    const value: i32 = await seven();

    console.log(letter, rendered, distinct.length, running.length);
    console.log(sorted[0], kept.length, shouted[0], same[0], value);
}

async function main(): Promise<void> {
    await probe();
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        // `scan` carries the seed, so four inputs give five outputs, and only
        // `alpha` is longer than four characters.
        "b {\"label\": \"x\",\"size\": 1} 3 5\nalpha 1 BETA beta 7\n"
    );
}

/// A tuple local can cross a suspension.
///
/// The predicate that decides whether a frame can own a value is written twice,
/// once for the type checker and once for code generation, and only the second
/// had a `Tuple` arm. So a `[string, i32]` local was refused from a frame with
/// a message saying the storage was not implemented, while every piece needed
/// to store one existed and answered yes on the other side.
#[test]
fn a_tuple_local_can_cross_an_await() {
    let project = AsyncProject::new(
        r#"async function seven(): Promise<i32> {
    return 7;
}

async function main(): Promise<void> {
    const pair: [string, i32] = ["alpha", 2];
    const nested: [string[], i32] = [["a", "b"], 3];
    const value: i32 = await seven();
    console.log(pair[0], pair[1] + value, nested[0][1], nested[1]);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "alpha 9 b 3\n");
}

/// A loop inside a `try` is refused by the checker, not by code generation.
///
/// It is refused either way: the lowering for that shape does not exist. What
/// changed is which message a program gets. The checker accepted it and code
/// generation then failed with byte offsets and an LLVM block name, which is a
/// dump rather than a diagnostic. Two answers to one question, and the one that
/// reached the reader was the wrong one.
///
/// The `help:` names both ways around it *and* the difference between them: a
/// `try` moved inside the loop compiles and also changes what the program does,
/// since a failure then ends one iteration instead of the loop. Saying only the
/// first half would be a trap.
#[test]
fn a_loop_awaiting_inside_a_try_is_refused_with_a_diagnostic() {
    let project = AsyncProject::new(
        r#"async function work(): Promise<void> {
    return;
}

async function main(): Promise<void> {
    try {
        let round: i32 = 0;
        while (round < 3) {
            await work();
            round = round + 1;
        }
    } catch (error: Error) {
        console.log(error.message);
    }
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the shape should be refused");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("E3002"),
        "the refusal should be a diagnostic, not a codegen dump:\n{stderr}"
    );
    assert!(
        !stderr.contains("in block Some("),
        "an LLVM block name reached the reader:\n{stderr}"
    );
    assert!(
        stderr.contains("move the `try` inside the loop"),
        "the refusal does not say what to do instead:\n{stderr}"
    );
    assert!(
        stderr.contains("changes the behaviour"),
        "the refusal suggests a fix without saying it means something different:\n{stderr}"
    );
}

/// An `await` inside a `try` nested in another `try` used to leave the
/// suspension unregistered: the outer statement produced no await sites when
/// its body had no direct suspension, and codegen failed with "await codegen is
/// not implemented for continuation value". The nested body now descends like
/// any other control-flow block, one `Try` context per level.
#[test]
fn an_await_inside_a_nested_try_suspends_and_resumes_in_place() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

async function fail(): Promise<i32> throws string {
    await sleepAsync(1);
    throw "boom";
}

export async function main(): Promise<void> {
    let count: i32 = 0;
    try {
        try {
            await sleepAsync(1);
            count = count + 1;
            await sleepAsync(1);
            count = count + 1;
        } catch (error: string) {
            console.log("inner", error);
        }
        count = count + 10;
        try {
            const value: i32 = await fail();
            console.log("unreachable", value);
        } catch (error: string) {
            console.log("caught", error);
        }
        count = count + 100;
    } catch (error: string) {
        console.log("outer", error);
    }
    console.log("count", count);
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
        "caught boom\ncount 112\n"
    );
}

/// Statements between two suspending statements of the same block run once.
/// Each suspending statement is lowered as its own path with the block around
/// it as context, and the statements between two paths were attached both as
/// the `after` of the first path and the `before` of the next, so they ran
/// twice: this loop printed `count 121`.
#[test]
fn statements_between_two_suspending_statements_run_once() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

export async function main(): Promise<void> {
    let count: i32 = 0;
    let round: i32 = 0;
    while (round < 1) {
        if (true) {
            await sleepAsync(1);
            count = count + 1;
        }
        count = count + 10;
        if (true) {
            await sleepAsync(1);
            count = count + 100;
        }
        round = round + 1;
    }
    console.log("count", count);
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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "count 111\n");
}

/// The module loader renames every type a project file declares, and the
/// `catch` clause's type was the one annotation it skipped. The generated code
/// then looked the original name up and did not find it: a synchronous `catch`
/// of an error interface the program declared made the compiler panic with
/// "unknown interface", and an `async` one refused to bind it as a structured
/// error.
#[test]
fn a_catch_names_an_error_interface_the_program_declared() {
    let project = AsyncProject::new(
        r#"import { sleepAsync } from "tyrus:task";

interface WriteError {
    code: string;
    message: string;
}

function failNow(): i32 throws WriteError {
    const failure: WriteError = { code: "ERR_WRITE_NOW", message: "cannot write now" };
    throw failure;
}

async function failLater(): Promise<i32> throws WriteError {
    await sleepAsync(1);
    const failure: WriteError = { code: "ERR_WRITE_LATER", message: "cannot write later" };
    throw failure;
}

export async function main(): Promise<void> {
    try {
        console.log(failNow());
    } catch (error: WriteError) {
        console.log("sync", error.code, error.message);
    }
    try {
        const value: i32 = await failLater();
        console.log("unreachable", value);
    } catch (error: WriteError) {
        console.log("async", error.code, error.message);
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
        String::from_utf8_lossy(&output.stdout),
        "sync ERR_WRITE_NOW cannot write now\nasync ERR_WRITE_LATER cannot write later\n"
    );
}

#[test]
fn writing_a_borrowed_byte_array_clones_the_stream_payload() {
    let project = AsyncProject::new(
        r#"import { StreamError, WritableStream } from "tyrus:stream";

async function writeBorrowed(stream: WritableStream<u8[]>, bytes: u8[]): Promise<void> throws StreamError {
    await stream.write(bytes);
}

async function main(): Promise<void> {}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    let ir = project.emitted_ir();
    let poll = async_poll_function(&ir, "__tyrus_module_0_writeBorrowed");
    assert!(
        poll.contains("call i32 @tyrus_array_clone_owned"),
        "a stream write must clone a borrowed array before dispatch:\n{poll}"
    );
    assert!(
        poll.contains("call ptr @tyrus_writable_stream_write"),
        "the cloned payload must be dispatched to the stream:\n{poll}"
    );
}

/// A TCP connection handed to `stream()` is a bounded byte duplex: closing the
/// writable side shuts down only the write direction, so the peer reads
/// end-of-file while this side can still read the answer, and the direct
/// `read`/`write` of the `TcpStream` return `ERR_NET_STREAMED` afterwards.
#[test]
fn a_tcp_stream_becomes_a_half_closable_byte_duplex() {
    if loopback_bind_is_denied() {
        return;
    }
    let project = AsyncProject::new(
        r#"import { NetError, TcpListener, TcpStream, tcpConnect, tcpListen } from "tyrus:net";
import { DuplexStream, StreamError, WritableStream } from "tyrus:stream";

async function exchange(client: DuplexStream<u8[], u8[]>, server: DuplexStream<u8[], u8[]>): Promise<string> throws StreamError {
    await client.writable.write([112, 105, 110, 103]);
    console.log("checkpoint tcp exchange: request written");
    await client.writable.close();
    console.log("checkpoint tcp exchange: client write closed");
    const request: u8[]? = await server.readable.read();
    console.log("checkpoint tcp exchange: request read");
    const serverEnd: u8[]? = await server.readable.read();
    console.log("checkpoint tcp exchange: server eof read");
    await server.writable.write([111, 107]);
    console.log("checkpoint tcp exchange: answer written");
    await server.writable.close();
    console.log("checkpoint tcp exchange: server write closed");
    const answer: u8[]? = await client.readable.read();
    console.log("checkpoint tcp exchange: answer read");
    const clientEnd: u8[]? = await client.readable.read();
    console.log("checkpoint tcp exchange: client eof read");
    console.log("request", request?.length ?? 0, "server eof", !serverEnd);
    console.log("answer", answer?.length ?? 0, "client eof", !clientEnd);
    return "done";
}

async function run(): Promise<Result<void, NetError>> {
    const listener: TcpListener = await tcpListen("127.0.0.1:0", 1000)?;
    console.log("checkpoint tcp run: listener created");
    const address: string = listener.localAddress();
    const client: TcpStream = await tcpConnect(address, 1000)?;
    console.log("checkpoint tcp run: client connected");
    const server: TcpStream = await listener.accept(1000)?;
    console.log("checkpoint tcp run: server accepted");
    const clientStream: DuplexStream<u8[], u8[]> = await client.stream()?;
    console.log("checkpoint tcp run: client stream created");
    const serverStream: DuplexStream<u8[], u8[]> = await server.stream()?;
    console.log("checkpoint tcp run: server stream created");
    try {
        const result: string = await exchange(clientStream, serverStream);
        console.log(result);
    } catch (error: StreamError) {
        console.log("stream", error.code);
    }
    match (await client.read(4)) {
        Ok(bytes) => { console.log("unreachable", bytes.length); }
        Err(error) => { console.log("direct", error.code); }
    }
    listener.close();
    return Ok();
}

export async function main(): Promise<void> {
    match (await run()) {
        Ok() => {}
        Err(error) => { console.log("net", error.code); }
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
    let ir = project.emitted_ir();
    let exchange_poll = async_poll_function(&ir, "__tyrus_module_0_exchange");
    assert_eq!(
        exchange_poll
            .matches("call void @tyrus_readable_stream_free")
            .count(),
        4,
        "every retained readable capability must be released:\n{exchange_poll}"
    );
    assert_eq!(
        exchange_poll
            .matches("call void @tyrus_writable_stream_free")
            .count(),
        4,
        "every retained writable capability must be released:\n{exchange_poll}"
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "program failed ({}):\nstdout:\n{}\nstderr:\n{}",
        process_status_description(&output.status),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "checkpoint tcp run: listener created\ncheckpoint tcp run: client connected\ncheckpoint tcp run: server accepted\ncheckpoint tcp run: client stream created\ncheckpoint tcp run: server stream created\ncheckpoint tcp exchange: request written\ncheckpoint tcp exchange: client write closed\ncheckpoint tcp exchange: request read\ncheckpoint tcp exchange: server eof read\ncheckpoint tcp exchange: answer written\ncheckpoint tcp exchange: server write closed\ncheckpoint tcp exchange: answer read\ncheckpoint tcp exchange: client eof read\nrequest 4 server eof true\nanswer 2 client eof true\ndone\ndirect ERR_NET_STREAMED\n"
    );
}

/// An `if` that tests an optional and awaits in its body, inside a loop, goes
/// through the recursive control lowering. That lowering used to read the
/// condition as a `boolean` only, which crashed the compiler on a `T?`, and to
/// open both branches without the narrowing the non-recursive lowering applies.
#[test]
fn an_optional_if_that_awaits_inside_a_loop_narrows_its_binding() {
    let project = AsyncProject::new(
        r#"interface Note {
    binary: boolean;
    text: string;
}

async function fetchNote(index: i32): Promise<Note?> {
    if (index < 2) {
        return { binary: false, text: "hi" };
    }
    return;
}

async function echo(text: string): Promise<void> {
    console.log("echo", text);
}

export async function main(): Promise<void> {
    let index: i32 = 0;
    let open: boolean = true;
    while (open) {
        const note: Note? = await fetchNote(index);
        if (note) {
            await echo(note.text);
        } else {
            open = false;
        }
        index = index + 1;
    }
    console.log("done", index);
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
        "echo hi\necho hi\ndone 3\n"
    );
}

#[test]
fn an_unannotated_catch_binds_the_thrown_error_code_and_message() {
    // `catch (error)` binds `Error`, and the code that was thrown survives both
    // a synchronous throw and the rejection of an awaited async function.
    let project = AsyncProject::new(
        r#"function failSynchronously(): void throws Error {
    const error: Error = { message: "sync failure", code: "ERR_SYNC" };
    throw error;
}

async function failAsynchronously(): Promise<void> throws Error {
    const error: Error = { message: "async failure", code: "ERR_ASYNC" };
    throw error;
}

async function main(): Promise<void> {
    try {
        failSynchronously();
    } catch (error) {
        console.log(error.code, error.message);
    }
    try {
        await failAsynchronously();
    } catch (error) {
        console.log(error.code, error.message);
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
        "ERR_SYNC sync failure\nERR_ASYNC async failure\n"
    );
    assert!(output.stderr.is_empty());
}

/// A thrown `Error` gives back both of its strings, not only the first.
///
/// The guard wrapper turns a `throw` that leaves an async function into a
/// rejected Promise. The rejection *copies* what it needs, so the thrown value
/// still owns its strings and the wrapper has to release them. It released
/// field zero alone, which let an `Error`'s `code` leak on every throw that
/// reached it, and the throw that reaches it is every rethrow out of a `catch`
/// that had suspended, because that binding is materialized by cloning both
/// strings.
///
/// Asserted on the emitted IR because the cost of getting this wrong is a leak,
/// which no assertion on a program's output can see.
#[test]
fn a_thrown_error_releases_its_code_as_well_as_its_message() {
    let project = AsyncProject::new(
        r#"async function step(): Promise<void> throws Error {
    const failure: Error = { message: "boom", code: "ERR_BOOM" };
    throw failure;
}

async function pause(): Promise<void> { return; }

async function rethrower(): Promise<void> throws Error {
    try {
        await step();
    } catch (error: Error) {
        await pause();
        throw error;
    }
}

export async function main(): Promise<void> {
    try {
        await rethrower();
    } catch (error: Error) {
        console.log(error.message, error.code);
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

    let ir = project.emitted_ir();
    let releases = ir.matches("guard.owned.field").count();
    assert!(
        releases >= 2,
        "the guard must give back every string the thrown value owns, found {releases} field reads:\n{ir}"
    );

    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "boom ERR_BOOM\n");
}

#[test]
fn for_of_over_non_copy_elements_works_with_and_without_an_await_in_the_body() {
    // The binding of an async `for...of` is a view of the element, as in a
    // synchronous loop, so it needs no Copy element: the loop once refused
    // strings and interfaces, even in a body that never suspends.
    let project = AsyncProject::new(
        r#"interface Paint {
    name: string;
    shade: i32;
}

async function main(): Promise<void> {
    const paints: Paint[] = [{ name: "sky", shade: 1 }, { name: "leaf", shade: 2 }, { name: "sun", shade: 3 }];
    let total: i32 = 0;
    for (const paint of paints) {
        total = total + paint.shade;
    }
    await Promise.resolve<void>();
    console.log(total);
    for (const paint of paints) {
        await Promise.resolve<void>();
        if (paint.shade === 3) {
            break;
        }
        console.log(paint.name);
    }
    const words: string[] = ["a".repeat(2), "b".repeat(2)];
    for (const word of words) {
        await Promise.resolve<void>();
        console.log(word.toUpperCase());
    }
    console.log(paints[2].name, words[1]);
}
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "6\nsky\nleaf\nAA\nBB\nsun bb\n"
    );
}
