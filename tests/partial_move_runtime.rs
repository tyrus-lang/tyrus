//! Moving a field out of a value, end to end.
//!
//! `const taken = order.items;` was refused with `E3081`, because nothing
//! tracked which part of `order` was gone and the program ended up with two
//! owners of one buffer. The field now moves: the new binding owns it, the rest
//! of the value stays usable, and the release of the whole value passes over
//! the moved field. Each program here would free a buffer twice, or read one
//! after freeing it, if any part of that were wrong.

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

struct MoveProject {
    root: PathBuf,
    name: String,
}

impl MoveProject {
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
        let name = format!("partial-move-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Partial move integration test" }},
  "security": {{ "envFile": ".env", "envSealed": false }},
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

    fn build_and_run(&self) -> Output {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed:\n{}",
            String::from_utf8_lossy(&build.stderr)
        );
        Command::new(self.root.join("target/debug").join(&self.name))
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for MoveProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_moved_field_belongs_to_the_new_binding_and_the_rest_stays_usable() {
    let project = MoveProject::new(
        r#"interface Basket {
    items: string[];
    label: string;
}

const main = (): void => {
    const suffix: string = "-" + "copy";
    const basket: Basket = { items: ["a" + suffix, "b"], label: "fruit" + suffix };
    let taken: string[] = basket.items;
    taken.push("c");
    console.log(taken.length);
    console.log(taken[0]);
    console.log(basket.label);
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "3\na-copy\nfruit-copy\n"
    );
}

#[test]
fn a_field_moved_on_one_branch_is_released_once_on_every_path() {
    let project = MoveProject::new(
        r#"interface Order {
    lines: string[];
    note: string;
}

const summarize = (flag: boolean): string => {
    const order: Order = { lines: ["one".toUpperCase(), "two"], note: "n".repeat(3) };
    if (flag) {
        const lines: string[] = order.lines;
        return lines[0];
    }
    return order.note;
};

const main = (): void => {
    console.log(summarize(true));
    console.log(summarize(false));
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "ONE\nnnn\n");
}

#[test]
fn a_field_written_again_after_a_move_is_released_once() {
    let project = MoveProject::new(
        r#"interface Basket {
    items: string[];
}

const main = (): void => {
    let basket: Basket = { items: ["a".toUpperCase()] };
    let round: i32 = 0;
    while (round < 3) {
        const taken: string[] = basket.items;
        console.log(taken[0]);
        basket.items = ["b".toUpperCase()];
        round += 1;
    }
    console.log(basket.items[0]);
};
"#,
    );
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "A\nB\nB\nB\n");
}

#[test]
fn a_move_repeated_by_a_loop_is_refused_before_it_can_run() {
    // This compiled and hung without printing anything: the second pass used,
    // and released again, the buffer the first pass had moved.
    let project = MoveProject::new(
        r#"const main = (): void => {
    const names: string[] = ["a", "b"];
    let round: i32 = 0;
    while (round < 2) {
        const taken: string[] = names;
        console.log(taken.length);
        round += 1;
    }
};
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("E0382") && stderr.contains("moved in the previous iteration of the loop"),
        "unexpected diagnostic: {stderr}"
    );
}
