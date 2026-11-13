//! End-to-end coverage for `tyrus:process` and its stream descriptors.

use std::fs;
use std::path::{Path, PathBuf};
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

fn generated_executable(root: &Path, name: &str) -> PathBuf {
    let mut executable = root.join("target/debug").join(name);
    if cfg!(windows) {
        executable.set_extension("exe");
    }
    executable
}

#[test]
fn process_streams_reach_the_generated_native_program() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("process-stream-runtime-{suffix}");
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
    "description": "Process stream integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    let child_expression = if cfg!(windows) {
        r#"exec("powershell.exe", ["-NoProfile", "-Command", "[Console]::OpenStandardOutput().Write([Text.Encoding]::UTF8.GetBytes('stream'),0,6); exit 3"])"#
    } else {
        r#"exec("sh", ["-c", "printf stream; exit 3"])"#
    };
    let source = r#"import { ChildProcess, ProcessError, exec } from "tyrus:process";
import { ReadableStream, WritableStream } from "tyrus:stream";

async function main(): Promise<void> {
    const child: ChildProcess = CHILD_EXPRESSION onfail {
        console.error(child.code);
        return;
    };
    const input: WritableStream<u8[]> = child.stdin;
    const output: ReadableStream<u8[]> = child.stdout;
    await input.close();
    const chunk: u8[]? = await output.read();
    console.log("length", chunk?.length ?? 0);
    console.log("assert:", 6);
    const wait: Result<i32, ProcessError> = await child.wait();
    match (wait) {
        Ok(code) => { console.log("exit", code); console.log("assert:", 3); }
        Err(error) => { console.error(error.code); }
    }
}
"#
    .replace("CHILD_EXPRESSION", child_expression);
    fs::write(root.join("src/main.tyr"), source).unwrap();
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
    let output = Command::new(generated_executable(&root, &name))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "length 6\nassert: 6\nexit 3\nassert: 3\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// A call borrows its arguments, including when the callee is `async`. The
/// async frame used to adopt the caller's `ChildProcess` reference and zero
/// the caller's slot, so `child.wait()` after `await inspect(child)` failed
/// with `ERR_PROCESS_CLOSED` even though the callee never touched the handle.
#[test]
fn a_child_process_passed_to_an_async_function_stays_usable_by_the_caller() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("process-async-borrow-{suffix}");
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
    "description": "Process handle borrow across an async call"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    let child_expression = if cfg!(windows) {
        r#"exec("powershell.exe", ["-NoProfile", "-Command", "exit 4"])"#
    } else {
        r#"exec("sh", ["-c", "exit 4"])"#
    };
    let source = r#"import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function inspect(child: ChildProcess): Promise<void> {
    console.log("pid positive", child.pid > 0);
}

async function ignore(child: ChildProcess): Promise<void> {
    console.log("ignored");
}

async function main(): Promise<void> {
    const child: ChildProcess = CHILD_EXPRESSION onfail {
        console.log("error", child.code);
        return;
    };
    await inspect(child);
    await ignore(child);
    const wait: Result<i32, ProcessError> = await child.wait();
    match (wait) {
        Ok(code) => { console.log("exit", code); }
        Err(error) => { console.log("error", error.code); }
    }
}
"#
    .replace("CHILD_EXPRESSION", child_expression);
    fs::write(root.join("src/main.tyr"), source).unwrap();
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
    let output = Command::new(generated_executable(&root, &name))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "pid positive true\nignored\nexit 4\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// `Promise.all` over two `pipeTo()` calls is the natural way to feed a child
/// process and drain it at the same time. The calls answer `Promise<void>`,
/// which codegen used to refuse twice: the array literal had no element type,
/// and a `void` payload had "no native size".
#[test]
#[cfg(unix)]
fn promise_all_over_pipe_to_relays_a_file_through_a_child_process() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("process-promise-all-relay-{suffix}");
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
    "description": "Promise.all relay through a child process"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    // Larger than the 256 KiB descriptor buffer, with every byte value present.
    let input: Vec<u8> = (0..600_000).map(|index| (index % 256) as u8).collect();
    fs::write(root.join("input.bin"), &input).unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { ChildProcess, ProcessError, exec } from "tyrus:process";
import { FsError, readFileStream, writeFileStream } from "tyrus:fs";
import { ReadableStream, StreamError, WritableStream } from "tyrus:stream";

async function relay(child: ChildProcess): Promise<Result<void, FsError>> {
    const source: ReadableStream<u8[]> = readFileStream("input.bin")?;
    const destination: WritableStream<u8[]> = writeFileStream("output.bin")?;
    const input: WritableStream<u8[]> = child.stdin;
    const output: ReadableStream<u8[]> = child.stdout;
    try {
        await Promise.all([source.pipeTo(input), output.pipeTo(destination)]);
    } catch (error: StreamError) {
        console.log("stream", error.code);
    }
    return Ok();
}

async function relayChild(child: ChildProcess): Promise<void> {
    match (await relay(child)) {
        Ok() => {}
        Err(error) => { console.log("fs", error.code); }
    }
    const wait = await child.wait();
    match (wait) {
        Ok(code) => { console.log("exit", code); }
        Err(error) => { console.log("process", error.code); }
    }
}

async function main(): Promise<void> {
    const child = exec("cat") onfail {
        console.log("process", child.code);
        return;
    };
    await relayChild(child);
}
"#,
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
    let output = Command::new(generated_executable(&root, &name))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "exit 0\n");
    assert_eq!(fs::read(root.join("output.bin")).unwrap(), input);
    let _ = fs::remove_dir_all(root);
}

/// A recoverable process error stays in the `Result` returned by an async
/// function, so awaiting the function does not reject its promise.
#[test]
fn a_process_error_keeps_its_code_across_an_async_boundary() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("process-async-error-code-{suffix}");
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
    "description": "Process error code across an async boundary"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function start(): Promise<Result<u32, ProcessError>> {
    const child: ChildProcess = exec("tyrus-command-that-does-not-exist-anywhere")?;
    return Ok(child.pid);
}

async function main(): Promise<void> {
    match (await start()) {
        Ok(pid) => { console.log("unreachable", pid); }
        Err(error) => { console.log("caught", error.code.startsWith("ERR_PROCESS_")); }
    }
}
"#,
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
    let output = Command::new(generated_executable(&root, &name))
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "caught true\n");
    let _ = fs::remove_dir_all(root);
}

/// Every child a generated program starts and waits for gives its three pipe
/// descriptors back. Counting `/proc/self/fd` before and after forty children
/// is what shows it: a handle that kept one descriptor per child would grow the
/// count by forty, far past the slack the runtime's own threads may take.
#[cfg(target_os = "linux")]
#[test]
fn a_generated_program_releases_the_descriptors_of_finished_children() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("process-descriptors-{suffix}");
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
    "description": "Process descriptor release test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { ChildProcess, ProcessError, exec } from "tyrus:process";
import { DirEntry, FsError, readdir } from "tyrus:fs";

function openDescriptors(): Result<usize, FsError> {
    const entries: DirEntry[] = readdir("/proc/self/fd")?;
    return Ok(entries.length);
}

async function runOnce(): Promise<Result<void, ProcessError>> {
    const child: ChildProcess = exec("true")?;
    const code: i32 = await child.wait()?;
    if (code !== 0) {
        console.log("exit", code);
    }
    return Ok();
}

async function runMany(count: i32): Promise<Result<void, ProcessError>> {
    let round: i32 = 0;
    while (round < count) {
        await runOnce()?;
        round = round + 1;
    }
    return Ok();
}

async function main(): Promise<void> {
    // One child first, so whatever the runtime opens once for the first child
    // (its worker threads, its signal pipe) is already counted in `before`.
    match (await runMany(1)) {
        Ok() => {}
        Err(error) => { console.error(error.code); }
    }
    let before: usize = 0;
    match (openDescriptors()) {
        Ok(count) => { before = count; }
        Err(error) => { console.error(error.code); }
    }
    match (await runMany(40)) {
        Ok() => {}
        Err(error) => { console.error(error.code); }
    }
    match (openDescriptors()) {
        Ok(after) => { console.log("grew", after as i64 - before as i64); }
        Err(error) => { console.error(error.code); }
    }
}
"#,
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
    let output = Command::new(generated_executable(&root, &name))
        .current_dir(&root)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(
        output.status.success(),
        "run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let grew: i64 = stdout
        .lines()
        .find_map(|line| line.strip_prefix("grew "))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("no descriptor count in:\n{stdout}"));
    assert!(
        grew < 10,
        "forty finished children grew the descriptor table by {grew}:\n{stdout}"
    );
}
