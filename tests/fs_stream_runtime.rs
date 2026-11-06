//! Native code generation coverage for filesystem stream constructors.

use std::fs;
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

#[test]
fn filesystem_stream_constructors_reach_the_native_runtime() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let name = format!("fs-stream-runtime-{suffix}");
    let root = std::env::temp_dir().join(format!("Tyrus Native {name} 東京"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Filesystem stream integration test"
  }},
  "dependencies": {{}}
}}
"#,
        ),
    )
    .unwrap();
    fs::write(root.join("input.bin"), [0_u8, 255, 1, 2]).unwrap();
    let input_path = if cfg!(windows) {
        r".\\..\\input.bin"
    } else {
        "./../input.bin"
    };
    let output_path = if cfg!(windows) {
        r".\\..\\output.bin"
    } else {
        "./../output.bin"
    };
    let source = r#"import { FsError, readFileStream, writeFileStream } from "tyrus:fs";

function main(): void {
    match (readFileStream("../input.bin")) {
        Ok(source) => {
            match (writeFileStream("../output.bin")) {
                Ok(destination) => { console.log("opened"); }
                Err(error) => { console.error(error.code); }
            }
        }
        Err(error) => { console.error(error.code); }
    }
}
"#
    .replace("../input.bin", input_path)
    .replace("../output.bin", output_path);
    fs::write(root.join("src/main.tyr"), source).unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut executable = root.join("target/debug").join(&name);
    if cfg!(windows) {
        executable.set_extension("exe");
    }
    let output = Command::new(executable)
        .current_dir(root.join("nested"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "opened\n");
    assert!(root.join("output.bin").exists());
    let _ = fs::remove_dir_all(root);
}
