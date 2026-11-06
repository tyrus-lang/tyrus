//! Every project `tyrus new` generates has to build as generated.
//!
//! The scaffold tests inspect the text of the files, which let both templates
//! ship code the compiler refuses: `process.env.TYRUS_ENV` bound to a local was
//! reported as a move out of a field, and so was `request.body` in the `api`
//! controller. Building the generated project is the only check that sees that.

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

struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(label: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(format!(
            "Tyrus Workspace 東京 with spaces/tyrus-new-{label}-{suffix}"
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn tyrus(&self, directory: &PathBuf, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .args(args)
            .current_dir(directory)
            .output()
            .unwrap()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn assert_success(output: &Output, step: &str) {
    assert!(
        output.status.success(),
        "{step} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn build_generated_project(target: &str) -> (Workspace, PathBuf) {
    let workspace = Workspace::new(target);
    let name = format!("generated-{target}");
    let created = workspace.tyrus(&workspace.root, &["new", &name, "--target", target]);
    assert_success(&created, "tyrus new");
    let created_stdout = String::from_utf8_lossy(&created.stdout);
    assert!(created_stdout.contains("    tyrus run\n"));
    assert!(!created_stdout.contains("    tyrus test\n"));
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(workspace.root.join(&name).join("tyrus.json")).unwrap())
            .unwrap();
    assert_eq!(config["scripts"]["test"], "tyrus test");
    let project = workspace.root.join(&name);
    let built = workspace.tyrus(&project, &["build"]);
    assert_success(&built, "tyrus build");
    let build_stdout = String::from_utf8_lossy(&built.stdout);
    let build_stderr = String::from_utf8_lossy(&built.stderr);
    assert!(!build_stdout.contains("Compiling "), "{build_stdout}");
    assert!(!build_stdout.contains("Finished →"), "{build_stdout}");
    assert!(build_stderr.contains("Compiling "), "{build_stderr}");
    assert!(build_stderr.contains("Finished →"), "{build_stderr}");
    (workspace, project)
}

#[test]
fn the_script_project_builds_and_runs_as_generated() {
    let (workspace, project) = build_generated_project("script");
    let run = workspace.tyrus(&project, &["run"]);
    assert_success(&run, "tyrus run");
    assert_eq!(
        String::from_utf8(run.stdout.clone()).unwrap(),
        "Hello World!\n"
    );
    let run_stderr = String::from_utf8_lossy(&run.stderr);
    assert!(run_stderr.contains("Compiling "), "{run_stderr}");
    assert!(run_stderr.contains("Finished →"), "{run_stderr}");

    let binary = project.join("target/debug").join("generated-script");
    let output = Command::new(binary).current_dir(&project).output().unwrap();
    assert_success(&output, "running the generated script");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "Hello World!\n");
}

#[test]
fn debug_and_release_builds_keep_separate_artifacts() {
    let (_workspace, project) = build_generated_project("script");
    let release = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["build", "--release"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert_success(&release, "tyrus build --release");
    for profile in ["debug", "release"] {
        let directory = project.join("target").join(profile);
        assert!(directory.join("generated-script").is_file());
        assert!(directory.join("generated-script.ll").is_file());
    }
}

#[test]
fn misspelled_release_flag_is_rejected() {
    let workspace = Workspace::new("unknown-build-flag");
    let output = workspace.tyrus(&workspace.root, &["build", "--realease"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown option `--realease`"));
}

#[test]
fn the_cli_project_builds_as_generated() {
    build_generated_project("cli");
}

#[test]
fn the_api_project_builds_as_generated() {
    let (_workspace, project) = build_generated_project("api");
    let lint = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["lint"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert_success(&lint, "linting the generated API project");
}
