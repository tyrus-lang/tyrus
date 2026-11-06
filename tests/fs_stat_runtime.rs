use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct StatProject {
    root: PathBuf,
    name: String,
}

impl StatProject {
    fn new() -> Self {
        let name = format!(
            "tyrus-fs-stat-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("present.txt"), "data").unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(r#"{{"project":{{"name":"{name}","version":"0.1.0"}},"dependencies":{{}}}}"#),
        )
        .unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { FileStats, FsError, stat } from "tyrus:fs";

const main = (): void => {
    const present: Result<FileStats, FsError> = stat("present.txt");
    match (present) {
        Ok(info) => { console.log(`ok:${info.size}`); }
        Err(error) => { console.log(error.code); }
    }
    const missing: Result<FileStats, FsError> = stat("missing.txt");
    match (missing) {
        Ok(_) => { console.log("unexpected success"); }
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

impl Drop for StatProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn stat_returns_metadata_or_a_structured_error() {
    let project = StatProject::new();
    let output = project.run();
    assert!(
        output.status.success(),
        "tyrus run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ok:4\nERR_FS_STAT_FAILED\n"
    );
    assert!(
        project
            .root
            .join("target/debug")
            .join(format!("{}{}", project.name, std::env::consts::EXE_SUFFIX))
            .exists()
    );
}
