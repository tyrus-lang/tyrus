use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn project_root(name: &str) -> std::path::PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("tyrus-{name}-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "isTTY runtime test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

fn build(root: &std::path::Path, binary_name: &str) -> std::path::PathBuf {
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );
    root.join("target/debug").join(binary_name)
}

/// A piped stdout is not a terminal: a program whose stdout is captured through
/// `.output()` must observe `isTTY() == false`. This is the deterministic half
/// of the contract and the one CI always exercises.
#[test]
fn is_tty_is_false_when_stdout_is_piped() {
    let root = project_root("cli-is-tty-piped");
    fs::write(
        root.join("src/main.tyr"),
        r#"import { isTTY } from "tyrus:cli";

const main = (): void => {
    const interactive: boolean = isTTY();
    console.log("interactive:", interactive);
};
"#,
    )
    .unwrap();
    let binary = build(&root, "cli-is-tty-piped");

    let output = Command::new(binary).output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "interactive: false\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}

/// With stdout attached to a real pseudoterminal, `isTTY()` must observe
/// `true`. `script(1)` (util-linux) runs the program under a pty; when the
/// helper is unavailable the assertion is skipped rather than forced.
#[test]
fn is_tty_is_true_when_stdout_is_a_terminal() {
    let script_version = Command::new("script").arg("--version").output();
    let has_script = script_version.is_ok_and(|output| output.status.success());
    if !has_script {
        eprintln!("skipping pty check: util-linux `script` not available");
        return;
    }

    let root = project_root("cli-is-tty-pty");
    fs::write(
        root.join("src/main.tyr"),
        r#"import { isTTY } from "tyrus:cli";

const main = (): void => {
    const interactive: boolean = isTTY();
    console.log("interactive:", interactive);
};
"#,
    )
    .unwrap();
    let binary = build(&root, "cli-is-tty-pty");
    let binary = binary.to_str().unwrap();

    // `-qec` stays quiet and propagates the child's exit status; the typescript
    // goes to /dev/null while the pty transcript streams to our stdout.
    let output = Command::new("script")
        .args(["-qec", binary, "/dev/null"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "pty run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let transcript = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(
        transcript.contains("interactive: true"),
        "expected `interactive: true` in the pty transcript, got: {transcript}"
    );

    fs::remove_dir_all(root).unwrap();
}
