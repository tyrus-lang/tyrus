use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct ClipboardReadProject {
    root: PathBuf,
    name: String,
}

impl ClipboardReadProject {
    fn new() -> Self {
        let name = format!(
            "tyrus-os-clipboard-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(r#"{{"project":{{"name":"{name}","version":"0.1.0"}},"dependencies":{{}}}}"#),
        )
        .unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { OsError, clipboardRead } from "tyrus:os";

const main = (): void => {
    const result: Result<string?, OsError> = clipboardRead();
    match (result) {
        Ok(text) => {
            if (text) { console.log(`text:${text.length}`); }
            else { console.log("empty"); }
        }
        Err(error) => { console.log(error.code); }
    }
};
"#,
        )
        .unwrap();
        Self { root, name }
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("run")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for ClipboardReadProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn clipboard_read_returns_absence_or_a_structured_error_without_corrupting_the_result_abi() {
    let project = ClipboardReadProject::new();
    let output = project.run();
    assert!(
        output.status.success(),
        "tyrus run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with("text:")
            || stdout == "empty\n"
            || stdout == "ERR_OS_CLIPBOARD_UNAVAILABLE\n"
            || stdout == "ERR_OS_CLIPBOARD_READ_FAILED\n",
        "unexpected clipboard result: {stdout:?}"
    );
    assert!(
        project
            .root
            .join("target/debug")
            .join(format!("{}{}", project.name, std::env::consts::EXE_SUFFIX))
            .exists()
    );
}
