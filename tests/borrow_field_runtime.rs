//! Borrowing a part of a value.
//!
//! `&order.items` borrows the field and keeps `order` borrowed: the reference
//! reads through that binding, so the binding is what has to stay put. Telling
//! two disjoint fields apart is a separate question this does not answer, and
//! the conservative answer is never wrong, only stricter than it needs to be.

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

struct BorrowProject {
    root: PathBuf,
    name: String,
}

impl BorrowProject {
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
        let name = format!("borrow-field-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "field borrow integration test" }},
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

    fn run(&self) -> Output {
        Command::new(self.root.join("target/debug").join(&self.name))
            .output()
            .unwrap()
    }

    fn build_and_run(&self) -> String {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = self.run();
        assert!(
            run.status.success(),
            "run failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
    }
}

impl Drop for BorrowProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_field_can_be_borrowed_and_read_through() {
    let project = BorrowProject::new(
        r#"
interface Basket {
    label: string;
    items: i32[];
}

function describe(value: &string): void {
    console.log(value);
}

function main(): void {
    const basket: Basket = { label: "fruit", items: [1, 2, 3] };
    describe(&basket.label);
    const borrowed: &string = &basket.label;
    describe(borrowed);
    console.log(basket.items.length);
}
"#,
    );
    assert_eq!(project.build_and_run(), "fruit\nfruit\n3\n");
}

#[test]
fn assigning_the_root_while_a_field_is_borrowed_is_refused() {
    // The reference reads through `basket`, so `basket` is what stays borrowed.
    let project = BorrowProject::new(
        r#"
interface Basket {
    label: string;
}

function describe(value: &string): void {
    console.log(value);
}

function main(): void {
    let basket: Basket = { label: "fruit" };
    const borrowed: &string = &basket.label;
    basket = { label: "other" };
    describe(borrowed);
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("E0506") && stderr.contains("while it is borrowed"),
        "unexpected diagnostic: {stderr}"
    );
}

#[test]
fn a_field_that_does_not_exist_is_reported_at_the_field() {
    let project = BorrowProject::new(
        r#"
interface Basket {
    label: string;
}

function describe(value: &string): void {
    console.log(value);
}

function main(): void {
    const basket: Basket = { label: "fruit" };
    describe(&basket.missing);
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("E0609") && stderr.contains("no field `missing`"),
        "unexpected diagnostic: {stderr}"
    );
}

#[test]
fn the_move_out_of_a_field_refusal_offers_the_borrow() {
    // A field of the same type moves; one narrowed out of its optional and bound
    // with the narrowed type would have to change shape where it stands, and
    // that refusal is where the borrow is offered.
    let project = BorrowProject::new(
        r#"
interface Basket {
    items?: i32[];
}

function main(): void {
    const basket: Basket = { items: [1, 2, 3] };
    if (basket.items) {
        const taken: i32[] = basket.items;
        console.log(taken.length);
    }
}
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("`&basket.items` to borrow it"),
        "the refusal does not offer the borrow: {stderr}"
    );
}
