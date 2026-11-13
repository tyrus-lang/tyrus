//! Native end-to-end coverage for `unknown` values and their `typeof` narrowing.
//!
//! An `unknown` is a tagged scalar at run time, so these programs check that the
//! category `typeof` reports, the narrowed branch the checker accepts and the
//! value read in that branch all agree, across every place a value can enter an
//! `unknown`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct UnknownProject {
    root: PathBuf,
    name: String,
}

impl UnknownProject {
    fn new(label: &str, source: &str) -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!("unknown-runtime-{label}-{suffix}");
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
    "description": "Unknown runtime integration test"
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

    fn stdout(&self) -> String {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed:\n{}{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let run = self.run();
        assert!(
            run.status.success(),
            "program failed with {:?}:\n{}",
            run.status,
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8(run.stdout).unwrap()
    }
}

impl Drop for UnknownProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn typeof_narrows_unknown_to_the_value_it_holds() {
    let project = UnknownProject::new(
        "narrowing",
        r#"const main = (): void => {
    const payload: unknown = "Tyrus";
    if (typeof payload === "string") {
        console.log(payload.length);
    }
    if ("string" === typeof payload) {
        console.log(payload.toUpperCase());
    }
    if (typeof payload !== "string") {
        console.log("unreachable");
    } else {
        console.log(payload.length);
    }
    if (typeof payload === "object") {
        console.log("unreachable");
    }
    console.log(payload);
};
"#,
    );
    assert_eq!(project.stdout(), "5\nTYRUS\n5\nTyrus\n");
}

#[test]
fn every_scalar_category_round_trips_through_unknown() {
    let project = UnknownProject::new(
        "categories",
        r#"const report = (value: unknown): void => {
    console.log(typeof value);
};

const main = (): void => {
    const letter: char = 'x';
    const tiny: i8 = -3;
    const short: i16 = 300;
    const long: i64 = 5000000000;
    const huge: i128 = 7;
    const offset: isize = -1;
    const byte: u8 = 200;
    const port: u16 = 8080;
    const count: u32 = 9;
    const id: u64 = 10;
    const mask: u128 = 11;
    const size: usize = 12;
    const ratio: f32 = 0.5;
    const extended: f128 = 1.5;
    report("text");
    report(true);
    report(letter);
    report(tiny);
    report(short);
    report(42);
    report(long);
    report(huge);
    report(offset);
    report(byte);
    report(port);
    report(count);
    report(id);
    report(mask);
    report(size);
    report(ratio);
    report(2.5);
    report(extended);
    report(1.25d);
    const held: unknown = tiny;
    if (typeof held === "i8") {
        console.log(held + 1);
    }
    const money: unknown = 1.25d;
    if (typeof money === "decimal") {
        console.log(money + 0.75d);
    }
};
"#,
    );
    assert_eq!(
        project.stdout(),
        "string\nboolean\nchar\ni8\ni16\ni32\ni64\ni128\nisize\nu8\nu16\nu32\nu64\nu128\nusize\nf32\nf64\nf128\ndecimal\n-2\n2\n"
    );
}

#[test]
fn unknown_values_cross_parameters_returns_assignments_globals_and_arrays() {
    let project = UnknownProject::new(
        "flow",
        r#"const topLevel: unknown = "global";

const echo = (value: unknown): unknown => value;

const main = (): void => {
    const back = echo("round");
    if (typeof back === "string") {
        console.log(back);
    }
    let slot: unknown = 1;
    slot = "now text";
    if (typeof slot === "string") {
        console.log(slot);
    }
    if (typeof topLevel === "string") {
        console.log(topLevel.length);
    }
    const shout: string = "dyn".toUpperCase();
    const owned: unknown = shout;
    if (typeof owned === "string") {
        console.log(owned);
    }
    const items: unknown[] = ["a", 2, false];
    for (const item of items) {
        console.log(typeof item);
    }
};
"#,
    );
    assert_eq!(
        project.stdout(),
        "round\nnow text\n6\nDYN\nstring\ni32\nboolean\n"
    );
}

#[test]
fn a_union_value_is_retagged_when_it_widens() {
    // Tags are positions in a union's variant list. Copying `string | i32` into
    // a wider union bit for bit read the stored `i32` back as a `string` and
    // dereferenced it, which crashed as soon as the value came from a parameter.
    let project = UnknownProject::new(
        "retag",
        r#"const widen = (value: string | i32): boolean | string | i32 => {
    const wide: boolean | string | i32 = value;
    return wide;
};

const hold = (value: string | i32): unknown => {
    const held: unknown = value;
    return held;
};

const main = (): void => {
    const number = widen(7);
    console.log(typeof number);
    console.log(number);
    const text = widen("hi");
    console.log(typeof text);
    console.log(text);
    const held = hold(41);
    if (typeof held === "i32") {
        console.log(held + 1);
    }
    const status: "active" | "inactive" = "inactive";
    const label: unknown = status;
    if (typeof label === "string") {
        console.log(label);
    }
};
"#,
    );
    assert_eq!(project.stdout(), "i32\n7\nstring\nhi\n42\ninactive\n");
}

#[test]
fn an_unknown_catch_receives_the_thrown_value_tagged() {
    let project = UnknownProject::new(
        "catch",
        r#"async function load(flag: boolean): Promise<string> {
    try {
        if (flag) {
            throw "async boom".toUpperCase();
        }
        return "fine";
    } catch (error: unknown) {
        if (typeof error === "string") {
            return error;
        }
        return "other";
    }
}

async function main(): Promise<void> {
    try {
        throw "boom";
    } catch (error: unknown) {
        if (typeof error === "string") {
            console.log(error);
        }
    }
    try {
        throw 404;
    } catch (failure: unknown) {
        if (typeof failure === "i32") {
            console.log(failure + 1);
        }
    }
    console.log(await load(true));
    console.log(await load(false));
}
"#,
    );
    assert_eq!(project.stdout(), "boom\n405\nASYNC BOOM\nfine\n");
}

#[test]
fn f128_literals_keep_precision_beyond_f64() {
    let project = UnknownProject::new(
        "f128-precision",
        r#"const main = (): void => {
    const slightlyGreater: f128 = 1.0000000000000000000000000000000002;
    const one: f128 = 1.0;
    console.log(slightlyGreater === one);
};
"#,
    );
    assert_eq!(project.stdout(), "false\n");
}

#[test]
fn an_f128_union_payload_is_stored_with_the_payload_alignment() {
    // The payload is an array of `i64` words, aligned to 8. A `fp128` stored
    // through it inherited its own 16-byte alignment, which emitted an aligned
    // SSE move into a slot that only guaranteed 8 and crashed on the first
    // assignment of an `f128` to any union containing one.
    let project = UnknownProject::new(
        "f128",
        r#"const report = (value: string | f128): void => {
    console.log(typeof value);
    console.log(value);
};

const main = (): void => {
    const extended: f128 = 1.5;
    report(extended);
    const local: string | f128 = extended;
    console.log(local);
    const held: unknown = extended;
    if (typeof held === "f128") {
        console.log(held * 2.0);
    }
};
"#,
    );
    assert_eq!(project.stdout(), "f128\n1.5\n1.5\n3.0\n");
}
