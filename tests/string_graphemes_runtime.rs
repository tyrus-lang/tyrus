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

fn project_root(name: &str) -> std::path::PathBuf {
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
    fs::write(
        root.join("tyrus.json"),
        format!(
            r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "string graphemes runtime test"
  }},
  "dependencies": {{}}
}}
"#
        ),
    )
    .unwrap();
    root
}

fn build_and_run(root: &std::path::Path, binary_name: &str) -> std::process::Output {
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
    Command::new(root.join("target/debug").join(binary_name))
        .output()
        .unwrap()
}

/// `graphemes()` counts what a reader calls a character; `chars()` counts
/// Unicode scalar values. The astronaut is the case that separates them: a man,
/// a zero-width joiner and a rocket render and delete as one thing.
///
/// Both are kept because both are needed. Indexing, slicing and `length` count
/// scalar values, since that is the unit the rest of the string API is measured
/// in; grapheme clusters answer the display question instead.
#[test]
fn graphemes_count_user_perceived_characters() {
    let root = project_root("string-graphemes");
    fs::write(
        root.join("src/main.tyr"),
        r#"function main(): void {
    const astronaut: string = "👨‍🚀";
    console.log(astronaut.chars().length);
    console.log(astronaut.graphemes().length);

    const mixed: string = "a👨‍🚀é";
    const parts: string[] = mixed.graphemes();
    console.log(parts.length);
    console.log(parts[0], parts[1], parts[2]);

    console.log("abc".graphemes().length);
    console.log("".graphemes().length);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "string-graphemes");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "3\n1\n3\na 👨\u{200d}🚀 é\n3\n0\n"
    );
    fs::remove_dir_all(root).unwrap();
}
