use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct FilesystemPathProject {
    root: PathBuf,
    name: String,
}

impl FilesystemPathProject {
    fn new() -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("tyrus-fs-paths-{suffix}");
        let root = std::env::temp_dir().join(format!("{name}-東京 space"));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("working/nested")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Filesystem relative path runtime test"
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
        fs::write(root.join("working/parent.txt"), "parent directory").unwrap();
        fs::write(root.join("working/native.txt"), "native separator").unwrap();
        fs::write(root.join("working/nested/current.txt"), "current directory").unwrap();
        fs::write(root.join("src/parent.txt"), "source directory decoy").unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { FsError, readFileSync } from "tyrus:fs";

const report = (path: string): void => {
    match (readFileSync(path)) {
        Ok(value) => { console.log(value); }
        Err(error) => { console.error(error.code); }
    }
};

const main = (): void => {
    report("../parent.txt");
    report("./current.txt");
    report("..\\native.txt");
};
"#,
        )
        .unwrap();
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
        let executable = self.root.join("target/debug").join(format!(
            "{}{}",
            self.name,
            std::env::consts::EXE_SUFFIX
        ));
        Command::new(executable)
            .current_dir(self.root.join("working/nested"))
            .output()
            .unwrap()
    }
}

impl Drop for FilesystemPathProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn relative_paths_resolve_from_the_process_working_directory() {
    let project = FilesystemPathProject::new();
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let run = project.run();
    assert!(
        run.status.success(),
        "program failed with {:?}:\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        "parent directory\ncurrent directory\nnative separator\n"
    );
}

#[test]
fn readdir_reports_structured_errors_without_returning_a_partial_listing() {
    let project = FilesystemPathProject::new();
    let file = project.root.join("not-a-directory");
    fs::write(&file, "file").unwrap();
    let file_literal = format!("{:?}", file.to_string_lossy().to_string());
    fs::write(
        project.root.join("src/main.tyr"),
        format!(
            r#"import {{ FsError, readdir }} from "tyrus:fs";

const main = (): void => {{
    match (readdir({file_literal})) {{
        Ok(_) => {{ console.error("unexpected success"); }}
        Err(error) => {{ console.log(error.code); }}
    }}
}};
"#
        ),
    )
    .unwrap();

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let run = project.run();
    assert!(
        run.status.success(),
        "program failed with {:?}:\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        "ERR_FS_READ_FAILED\n"
    );
    assert!(run.stderr.is_empty(), "unexpected stderr: {:?}", run.stderr);
}

#[test]
fn readlink_reports_a_structured_error_for_a_non_link_path() {
    let project = FilesystemPathProject::new();
    let path = project.root.join("missing-link");
    let path_literal = format!("{:?}", path.to_string_lossy().to_string());
    fs::write(
        project.root.join("src/main.tyr"),
        format!(
            r#"import {{ FsError, readlink }} from "tyrus:fs";

const main = (): void => {{
    match (readlink({path_literal})) {{
        Ok(_) => {{ console.error("unexpected success"); }}
        Err(error) => {{ console.log(error.code); }}
    }}
}};
"#
        ),
    )
    .unwrap();

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let run = project.run();
    assert!(
        run.status.success(),
        "program failed with {:?}:\n{}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8(run.stdout).unwrap(),
        "ERR_FS_READLINK_FAILED\n"
    );
}
