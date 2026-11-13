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
    "description": "literal property access runtime test"
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

/// `obj["name"]` is `obj.name`: the key is written in the source, so it names a
/// field and compiles to the same fixed offset the dot form uses.
///
/// The dynamic form stays refused (`obj[key]` is `E0608`) and `Record<string, T>`
/// keeps its run-time lookup, which is why both appear here, the rule has to
/// tell a struct apart from a hash map, and getting that wrong in either
/// direction is silent.
#[test]
fn a_literal_key_reads_and_writes_a_struct_field() {
    let root = project_root("literal-property-access");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Person {
    name: string;
    age: i32;
}

function main(): void {
    let person: Person = { name: "Ada", age: 36 };
    console.log(person["name"], person["age"]);

    person["age"] = 42;
    person["name"] = "Grace";
    console.log(person.name, person.age);

    const inline: { title: string; year: i32 } = { title: "Book", year: 1999 };
    console.log(inline["title"], inline["year"]);

    const lookup: Record<string, i32> = { a: 1 };
    console.log(lookup["a"] ?? -1);
    console.log(lookup["missing"] ?? -1);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "literal-property-access");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Ada 36\nGrace 42\nBook 1999\n1\n-1\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn object_pick_and_omit_build_independent_structures() {
    let root = project_root("object-pick-omit");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface User {
    name: string;
    age: i32;
    nickname?: string;
    tags: string[];
}

type PublicKey = "name" | "nickname" | "tags";

function main(): void {
    let user: User = { name: "Ada", age: 36, tags: ["compiler"] };
    const publicKeys: PublicKey[] = ["name", "nickname", "tags"];
    const picked = user.pick(publicKeys);
    const omitted = user.omit(["age"]);
    const empty = user.pick([]);
    const chained = user.pick(["name", "nickname"]).omit(["nickname"]);

    user.age = 37;
    console.log(user.age, picked.name, picked.nickname ?? "missing", picked.tags[0]);
    console.log(omitted.name, omitted.tags[0]);
    console.log(chained.name);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "object-pick-omit");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "37 Ada missing compiler\nAda compiler\nAda\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn automatic_data_methods_copy_compare_and_render_nested_values() {
    let root = project_root("automatic-data-methods");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Account {
    owner: string;
    nickname?: string;
    balance: decimal;
}

function main(): void {
    const first: Account = { owner: "Ada", balance: 12.50d };
    printError("automatic data methods: before copy");
    const copied: Account = first.copy();
    printError("automatic data methods: after copy");
    const different: Account = { owner: "Ada", nickname: "A", balance: 12.5d };

    printError("automatic data methods: before equals copied");
    console.log(first.equals(copied));
    printError("automatic data methods: after equals copied");
    printError("automatic data methods: before equals different");
    console.log(first.equals(different));
    printError("automatic data methods: after equals different");
    printError("automatic data methods: before toString");
    console.log(first.toString());
    printError("automatic data methods: after toString");
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "automatic-data-methods");
    assert!(
        output.status.success(),
        "generated program exited with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "true\nfalse\n{ owner: \"Ada\", nickname: None, balance: 12.5 }\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// Member access on an optional field a narrowing has already unwrapped.
///
/// Inside `if (inner.items)` the value is a `string[]`, so `.length` is the
/// array's. The emitter read the declared type instead, asked for a field of an
/// optional, and refused with "field access requires an object value": the
/// expression did not compile at all, at any nesting depth.
#[test]
fn a_narrowed_optional_field_exposes_its_payload_members() {
    let root = project_root("narrowed-field-members");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
interface Inner {
    tag?: string;
    items?: string[];
}

interface Outer {
    inner: Inner;
}

const main = (): void => {
    const outer: Outer = { inner: { tag: "ada", items: ["a", "b"] } };

    if (outer.inner.items) {
        console.log(outer.inner.items.length);
    }
    if (outer.inner.tag) {
        console.log(outer.inner.tag.length);
        console.log(outer.inner.tag.toUpperCase());
    }

    const inner: Inner = { tag: "bob", items: ["x"] };
    if (inner.items) {
        console.log(inner.items.length);
    }
};
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "narrowed-field-members");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "2\n3\nADA\n1\n");
    fs::remove_dir_all(root).unwrap();
}
