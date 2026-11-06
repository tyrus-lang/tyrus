//! Decoding a value back out of JSON, end to end.
//!
//! These run the produced binary. The decode path had only IR-level coverage,
//! which is how a stack overwrite in the narrowed binding went unnoticed: the
//! IR asserted the right runtime calls were emitted, and the program still
//! crashed the moment it ran.

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

struct JsonProject {
    root: PathBuf,
    name: String,
}

impl JsonProject {
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
        let name = format!("json-decode-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "json decode integration test" }},
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
        String::from_utf8_lossy(&run.stdout).into_owned()
    }
}

impl Drop for JsonProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_narrowed_result_binds_its_payload_and_not_the_whole_tagged_value() {
    // `const parsed: User = decoded;` inside `if (decoded)` used to store every
    // byte of the `Result` into a slot sized for the payload. That is a stack
    // overwrite: the function returned into whatever it left behind, and the
    // program died with SIGSEGV before printing anything.
    let project = JsonProject::new(
        r#"
interface User {
    name: string;
    count: i32;
}

function main(): void {
    const decoded = User.fromJson("{\"name\":\"Ada\",\"count\":3}");
    if (decoded) {
        const parsed: User = decoded;
        console.log(parsed.name);
        console.log(parsed.count);
    } else {
        console.log("failed");
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "Ada\n3\n");
}

#[test]
fn a_round_trip_through_json_preserves_every_field() {
    let project = JsonProject::new(
        r#"
interface Snapshot {
    name: string;
    nickname?: string;
    balance: decimal;
}

function main(): void {
    const original: Snapshot = { name: "Ada", balance: 12.50d };
    const decoded = Snapshot.fromJson(original.toJson());
    if (decoded) {
        const parsed: Snapshot = decoded;
        console.log(parsed.name);
        console.log(parsed.balance);
    } else {
        console.log("failed");
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "Ada\n12.5\n");
}

#[test]
fn a_nested_array_survives_the_round_trip() {
    let project = JsonProject::new(
        r#"
interface Address {
    city: string;
}

interface Directory {
    entries: Address[];
}

function main(): void {
    const original: Directory = { entries: [{ city: "Ada" }, { city: "Grace" }] };
    const decoded = Directory.fromJson(original.toJson());
    if (decoded) {
        const parsed: Directory = decoded;
        console.log(parsed.entries[1].city);
    } else {
        console.log("failed");
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "Grace\n");
}

#[test]
fn a_decimal_is_written_in_the_same_notation_it_prints_in() {
    // The wire used to carry `125e-1` for a value that printed as `12.5`: legal
    // JSON, read back as the same number, but the same value spelled two ways
    // depending on where you looked at it.
    let project = JsonProject::new(
        r#"
interface Priced {
    balance: decimal;
    big: decimal;
    small: decimal;
    whole: decimal;
}

function main(): void {
    const value: Priced = { balance: 12.50d, big: 1200d, small: 0.005d, whole: 7d };
    console.log(value.toJson());
    const decoded = Priced.fromJson(value.toJson());
    if (decoded) {
        const parsed: Priced = decoded;
        console.log(parsed.balance);
        console.log(parsed.small);
    } else {
        console.log("failed");
    }
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "{\"balance\": 12.5,\"big\": 1200,\"small\": 0.005,\"whole\": 7}\n12.5\n0.005\n"
    );
}

#[test]
fn typeof_narrows_a_decimal_out_of_a_union() {
    // The type checker accepts `typeof value === "decimal"` as a narrowing, but
    // the runtime answered "unknown" for a decimal variant, so the comparison
    // was always false: the branch the compiler proved reachable never ran and
    // nothing reported an error.
    let project = JsonProject::new(
        r#"
function main(): void {
    const amount: decimal = 12.50d;
    console.log(typeof amount);
    const value: string | decimal = 12.50d;
    if (typeof value === "decimal") {
        const parsed: decimal = value;
        console.log(parsed);
    } else {
        console.log("took the wrong branch");
    }
    const text: string | decimal = "hi";
    if (typeof text === "decimal") {
        console.log("took the wrong branch");
    } else {
        const parsed: string = text;
        console.log(parsed);
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "decimal\n12.5\nhi\n");
}

#[test]
fn typeof_answers_with_the_tyrus_type_name() {
    // A tyrus type answers with its own name, not with JavaScript's collapsed
    // categories: `[1, 2]` is `i32[]`, the same way `42` is `i32` and not
    // `number`. Only an anonymous object type and a function answer with a
    // category, having no name to give.
    let project = JsonProject::new(
        r#"
interface Point {
    x: i32;
}

function main(): void {
    const numbers: i32[] = [1, 2];
    const nested: string[][] = [["a"]];
    const point: Point = { x: 1 };
    const maybe: string? = "x";
    console.log(typeof 42);
    console.log(typeof numbers);
    console.log(typeof nested);
    console.log(typeof point);
    console.log(typeof maybe);
    console.log(typeof 12.50d);
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "i32\ni32[]\nstring[][]\nPoint\nstring?\ndecimal\n"
    );
}

#[test]
fn typeof_narrows_a_union_by_the_name_it_reports() {
    // The string the runtime produces and the string the checker accepts come
    // from one table now, so a name that prints is a name that narrows.
    let project = JsonProject::new(
        r#"
function main(): void {
    const value: string | i32[] = [1, 2];
    console.log(typeof value);
    if (typeof value === "i32[]") {
        const numbers: i32[] = value;
        console.log(numbers.length);
    } else {
        console.log("took the wrong branch");
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "i32[]\n2\n");
}

#[test]
fn a_decode_failure_takes_the_error_branch_without_crashing() {
    let project = JsonProject::new(
        r#"
interface User {
    name: string;
    count: i32;
}

function main(): void {
    const decoded = User.fromJson("{}");
    if (decoded) {
        console.log("unexpected");
    } else {
        console.log("refused");
    }
}
"#,
    );
    assert_eq!(project.build_and_run(), "refused\n");
}
