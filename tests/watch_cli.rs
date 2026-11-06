//! End-to-end coverage for `tyrus watch`.

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn temp_root(name: &str) -> std::path::PathBuf {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-{name}-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    root
}

/// A watcher that is killed however the test ends, so a failing assertion does
/// not leave it observing a deleted directory.
struct Watcher(std::process::Child);

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn watch_refuses_a_directory_without_a_project() {
    let root = temp_root("watch-no-project");
    let output = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("watch")
        .current_dir(&root)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&root);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("tyrus.json"),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn watch_reruns_its_command_for_sources_but_not_for_other_or_ignored_files() {
    let root = temp_root("watch-command");
    fs::write(
        root.join("tyrus.json"),
        r#"{ "project": { "name": "watched", "version": "0.1.0", "authors": [], "description": "watch test" }, "dependencies": {} }"#,
    )
    .unwrap();
    fs::write(root.join(".tyrusignore"), "src/generated\n").unwrap();
    fs::create_dir_all(root.join("src/generated")).unwrap();
    fs::write(root.join("src/main.tyr"), "console.log(\"one\");\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(["watch", "--command", "echo tick", "--debounce", "100"])
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let watcher = Watcher(child);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let next_tick = |within: Duration| -> bool {
        let deadline = std::time::Instant::now() + within;
        while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
            match receiver.recv_timeout(remaining) {
                Ok(line) if line.trim() == "tick" => return true,
                Ok(_) => {}
                Err(_) => return false,
            }
        }
        false
    };

    // The first cycle runs as soon as watching starts.
    assert!(next_tick(Duration::from_secs(20)), "no initial run");
    // Give the watcher a moment to register before touching files.
    std::thread::sleep(Duration::from_millis(300));

    fs::write(root.join("src/notes.txt"), "not a source\n").unwrap();
    fs::write(
        root.join("src/generated/api.tyr"),
        "console.log(\"gen\");\n",
    )
    .unwrap();
    assert!(
        !next_tick(Duration::from_millis(1500)),
        "a non-source file or an ignored path started a cycle"
    );

    fs::write(root.join("src/main.tyr"), "console.log(\"two\");\n").unwrap();
    let rerun = next_tick(Duration::from_secs(20));
    drop(watcher);
    let _ = fs::remove_dir_all(&root);
    assert!(rerun, "changing a source did not start a cycle");
}
