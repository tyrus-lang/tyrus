use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

struct RevealInFolderProject {
    root: PathBuf,
    name: String,
}

impl RevealInFolderProject {
    fn new() -> Self {
        let name = format!(
            "tyrus-os-reveal-{}-{}",
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
            format!(r#"{{"project":{{"name":"{name}","version":"0.1.0"}}}}"#),
        )
        .unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { OsError, revealInFolder } from "tyrus:os";

const reveal = (path: string): Result<void, OsError> => revealInFolder(path);
const main = (): void => {};
"#,
        )
        .unwrap();
        Self { root, name }
    }

    fn build(&self) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for RevealInFolderProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn reveal_in_folder_result_is_available_to_generated_native_code() {
    let project = RevealInFolderProject::new();
    let output = project.build();
    assert!(
        output.status.success(),
        "tyrus build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        project
            .root
            .join("target/debug")
            .join(format!("{}{}", project.name, std::env::consts::EXE_SUFFIX))
            .exists()
    );
}
