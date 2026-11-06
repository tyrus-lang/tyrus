//! Native end-to-end coverage for `Map`/`Set` construction when the type
//! arguments are omitted and have to come from the annotation or from the
//! initializer.

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

struct CollectionProject {
    root: PathBuf,
    name: String,
}

impl CollectionProject {
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
        let name = format!("collection-runtime-{suffix}");
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
    "description": "Collection runtime integration test"
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

impl Drop for CollectionProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn run_expecting(source: &str, expected_stdout: &str) {
    let project = CollectionProject::new(source);
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = project.run();
    assert!(
        output.status.success(),
        "program failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected_stdout);
    assert!(output.stderr.is_empty());
}

/// `Map([[k, v], ...])` names its own types through its entries, the way
/// `Set([...])` always has. Writing them again in `Map<string, i32>(...)` next
/// to entries that already say `string` and `i32` is the redundancy this
/// removes; the codegen has to reach the same two types the checker did, or
/// the program builds against the wrong key width.
#[test]
fn a_map_takes_its_types_from_its_entries() {
    run_expecting(
        r#"function main(): void {
    const scores = Map([["ada", 1], ["grace", 2]]);
    console.log(scores.size);
    console.log(scores.get("grace") ?? 0);

    const rates = Map([[1, 0.5], [2, 1.5]]);
    console.log(rates.get(2) ?? 0.0);

    // A branch has no annotation to fall back on, so each arm has to build its
    // own types from its own entries.
    const branch = true;
    const picked = branch ? Map([["yes", 1]]) : Map([["no", 2]]);
    console.log(picked.get("yes") ?? 0);
}
"#,
        "2\n2\n1.5\n1\n",
    );
}

/// An annotation outranks the entries: `Map<string, f64>` seeded with `1`
/// holds `1.0`, not an `i32` reinterpreted at the wrong width.
#[test]
fn an_annotation_outranks_the_entry_types() {
    run_expecting(
        r#"function main(): void {
    const rates: Map<string, f64> = Map([["half", 1]]);
    console.log(rates.get("half") ?? 0.0);

    const marks: Set<f64> = Set([1, 2]);
    console.log(marks.has(1.0));
}
"#,
        "1.0\ntrue\n",
    );
}

/// An empty `Map()`/`Set()` has no entries to read, so the annotation is the
/// only source of types, including when it arrives through a parameter, a
/// return type, or a type alias rather than a binding.
#[test]
fn an_empty_collection_takes_its_types_from_the_context() {
    run_expecting(
        r#"type Counters = Map<string, i32>;

function fill(counters: Map<string, i32>): usize {
    counters.set("a", 1);
    return counters.size;
}

function empty(): Set<i32> {
    return Set();
}

function main(): void {
    const counters: Counters = Map();
    counters.set("seen", 3);
    console.log(counters.get("seen") ?? 0);

    console.log(fill(Map()));
    console.log(empty().size);
}
"#,
        "3\n1\n0\n",
    );
}
