//! Native end-to-end coverage for the `panic(message)` built-in
//! (`../tyrus-roadmaps/panic.md`): reports on stderr, exits non-zero, and
//! never reaches the code after it.

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

struct PanicProject {
    root: PathBuf,
    name: String,
}

impl PanicProject {
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
        let name = format!("panic-runtime-{suffix}");
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
    "description": "panic() runtime integration test"
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

impl Drop for PanicProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn panic_writes_the_message_to_stderr_exits_non_zero_and_skips_later_code() {
    let project = PanicProject::new(
        r#"function validate(age: i32): void {
    if (age < 0) {
        panic("age cannot be negative");
    }
    console.log("valid age");
}

function main(): void {
    validate(5);
    validate(-5);
    console.log("unreachable");
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
        !output.status.success(),
        "expected a non-zero exit status after `panic`"
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "valid age\n",
        "code after the failing `validate` call must never run"
    );
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "age cannot be negative\n"
    );
}

#[test]
fn panic_accepts_a_computed_message() {
    let project = PanicProject::new(
        r#"function main(): void {
    const reason: string = "boom " + "!".repeat(3);
    panic(reason);
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
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(String::from_utf8(output.stderr).unwrap(), "boom !!!\n");
}
