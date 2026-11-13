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
    "description": "copy() runtime test"
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
/// `copy()` produces a value with its own buffer.
///
/// The point is what the source does afterwards: a copy that shared the buffer
/// would show every mutation of the duplicate, which is the aliasing that made
/// a field read a double free. Every pair below is printed with the value the
/// source must still have.
#[test]
fn copy_produces_an_independent_value() {
    let root = project_root("copy-independent");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
interface Order {
    items: string[];
    name: string;
    pair: [i32, string];
}

const main = (): void => {
    const order: Order = { items: ["book", "pen"], name: "ada", pair: [1, "x"] };

    let lines: string[] = order.items.copy();
    lines.push("bag");
    console.log("lines", lines.length, "items", order.items.length);
    console.log("assert:", "lines", 3, "items", 2);

    const name: string = order.name.copy();
    console.log("name", name, "source", order.name);
    console.log("assert:", "name", "ada", "source", "ada");

    const pair: [i32, string] = order.pair.copy();
    console.log("pair", pair[0], pair[1]);
    console.log("assert:", "pair", 1, "x");

    let numbers: i32[] = [1, 2];
    let more: i32[] = numbers.copy();
    more.push(3);
    console.log("more", more.length, "numbers", numbers.length);
    console.log("assert:", "more", 3, "numbers", 2);
};
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "copy-independent");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Each printed line is followed by the value it must equal, so a mismatch
    // names both halves instead of only saying a count was wrong.
    let lines: Vec<&str> = stdout.lines().collect();
    for pair in lines.chunks(2) {
        let [produced, expected] = pair else { continue };
        let expected = expected.trim_start_matches("assert: ");
        assert_eq!(
            *produced, expected,
            "a copy shared its source's buffer:\n{stdout}"
        );
    }
    assert_eq!(stdout.matches("assert:").count(), 4);
}

/// `copy()` on a field a narrowing has already unwrapped.
///
/// Inside `if (outer.inner.tag)` the value is a `string`, not a `string?`, but
/// the type the compiler had recorded for the path still said optional. Cloning
/// it while believing it was an optional read the string's first word as a
/// pointer and crashed the compiler, on the exact expression the `E3081`
/// `help:` tells the reader to write.
#[test]
fn copy_works_on_a_narrowed_optional_field() {
    let root = project_root("copy-narrowed");
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

    if (outer.inner.tag) {
        const copied: string = outer.inner.tag.copy();
        console.log("copied", copied);
        console.log("assert:", "copied", "ada");
        console.log("source", outer.inner.tag);
        console.log("assert:", "source", "ada");
    }

    if (outer.inner.items) {
        let list: string[] = outer.inner.items.copy();
        list.push("c");
        console.log("list", list.length, "source", outer.inner.items.length);
        console.log("assert:", "list", 3, "source", 2);
    }
};
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "copy-narrowed");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<&str> = stdout.lines().collect();
    for pair in lines.chunks(2) {
        let [produced, expected] = pair else { continue };
        let expected = expected.trim_start_matches("assert: ");
        assert_eq!(
            *produced, expected,
            "a copy of a narrowed field was wrong:\n{stdout}"
        );
    }
    assert_eq!(stdout.matches("assert:").count(), 3);
}

/// `copy()` of an optional field keeps the value it held.
///
/// The clone was boxed a second time: `copy()` already returns an optional, but
/// nothing recorded that, so binding it to a `T?` wrapped the clone's box in
/// another box. Reading it then found a pointer where a value should be and
/// reported the optional as empty. Nothing failed and nothing was reported: the
/// value simply disappeared, on the expression the `E3081` `help:` recommends,
/// which is also the only way to bind an optional field at all.
#[test]
fn copy_of_an_optional_field_keeps_its_value() {
    let root = project_root("copy-optional");
    fs::write(
        root.join("src").join("main.tyr"),
        r#"
interface Profile {
    nickname?: string;
    scores?: i32[];
}

const main = (): void => {
    const named: Profile = { nickname: "ada", scores: [1, 2] };
    const anonymous: Profile = { };

    const nickname: string? = named.nickname.copy();
    console.log("present", nickname ?? "gone");
    console.log("assert:", "present", "ada");
    console.log("length", nickname?.length ?? 0);
    console.log("assert:", "length", 3);

    const missing: string? = anonymous.nickname.copy();
    console.log("absent", missing ?? "gone");
    console.log("assert:", "absent", "gone");

    const scores: i32[]? = named.scores.copy();
    console.log("scores", scores?.length ?? 0);
    console.log("assert:", "scores", 2);
};
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "copy-optional");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "program failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let lines: Vec<&str> = stdout.lines().collect();
    for pair in lines.chunks(2) {
        let [produced, expected] = pair else { continue };
        let expected = expected.trim_start_matches("assert: ");
        assert_eq!(
            *produced, expected,
            "a copied optional lost its value:\n{stdout}"
        );
    }
    assert_eq!(stdout.matches("assert:").count(), 4);
}
