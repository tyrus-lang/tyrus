use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Same scheme the other runtime suites use: a name built only from a timestamp
/// collided whenever two tests reached it inside the same tick, and the loser
/// lost its binary to the winner's cleanup.
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
    "description": "type intersection runtime test"
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

/// An intersection alias is a nominal type by the time codegen sees it, so the
/// three written forms have to reach the same flat layout: named operands, an
/// inline object as the first operand, and a chain longer than two.
#[test]
fn an_intersection_alias_lays_out_every_operand_field() {
    let root = project_root("type-intersection");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface B {
    id: i32;
}

interface C {
    age: i32;
}

type Named = B & C;
type WithInline = { name: string; } & B;
type Chained = { name: string; } & B & C;

function main(): void {
    const named: Named = { id: 1, age: 30 };
    console.log(named.id, named.age);

    const inline: WithInline = { id: 2, name: "Ada" };
    console.log(inline.id, inline.name);

    const chained: Chained = { id: 3, name: "Grace", age: 45 };
    console.log(chained.id, chained.name, chained.age);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "type-intersection");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1 30\n2 Ada\n3 Grace 45\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// The type checker is never handed the declaring module's items, so an export
/// has to cross the boundary already flattened. An intersection alias keeps all
/// of its operands in `extends`, which makes it the strictest case: exported raw
/// it would arrive with no fields at all.
#[test]
fn an_exported_intersection_alias_crosses_a_module_boundary() {
    let root = project_root("type-intersection-module");
    fs::write(
        root.join("src/shapes.tyr"),
        r#"export interface Identified {
    id: i32;
}

export interface Tagged {
    tag: string;
}

export type Labeled = { label: string; } & Identified;
export type Both = Identified & Tagged;
export interface Inherited extends Identified {
    extra: i32;
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { Both, Inherited, Labeled } from "./shapes";

function main(): void {
    const labeled: Labeled = { id: 7, label: "seven" };
    console.log(labeled.id, labeled.label);

    const both: Both = { id: 8, tag: "eight" };
    console.log(both.id, both.tag);

    const inherited: Inherited = { id: 9, extra: 90 };
    console.log(inherited.id, inherited.extra);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "type-intersection-module");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "7 seven\n8 eight\n9 90\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// An interface declared here that extends an imported one used to lose every
/// field, its own included, because the local type definitions never learned
/// about the imported supertype.
#[test]
fn a_local_type_extends_an_imported_interface() {
    let root = project_root("type-intersection-local-extends");
    fs::write(
        root.join("src/shapes.tyr"),
        r#"export interface Identified {
    id: i32;
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { Identified } from "./shapes";

interface Labeled extends Identified {
    label: string;
}

type Combined = { extra: i32; } & Identified;

function main(): void {
    const labeled: Labeled = { id: 1, label: "one" };
    console.log(labeled.id, labeled.label);

    const combined: Combined = { id: 2, extra: 20 };
    console.log(combined.id, combined.extra);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "type-intersection-local-extends");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1 one\n2 20\n");
    fs::remove_dir_all(root).unwrap();
}

/// The alias desugars into an interface declaration, so it takes type parameters
/// like any other, and the operands are substituted before the fields merge.
#[test]
fn a_generic_intersection_alias_substitutes_its_type_argument() {
    let root = project_root("type-intersection-generic");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Box<T> {
    value: T;
}

interface Tagged {
    tag: string;
}

type TaggedBox<T> = Box<T> & Tagged;

function main(): void {
    const boxed: TaggedBox<i32> = { value: 7, tag: "seven" };
    console.log(boxed.value, boxed.tag);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "type-intersection-generic");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "7 seven\n");
    fs::remove_dir_all(root).unwrap();
}

/// An intersection is symmetric, so a field that one side requires is required
/// whichever order the operands are written in, and a field one side freezes
/// stays read-only. Inheritance keeps its own directional rule.
#[test]
fn intersection_modifiers_are_symmetric_at_run_time() {
    let root = project_root("type-intersection-modifiers");
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Loose {
    tag?: string;
    readonly id: i32;
}

interface Strict {
    tag: string;
    id: i32;
}

type Forward = Loose & Strict;
type Backward = Strict & Loose;

function main(): void {
    const forward: Forward = { tag: "a", id: 1 };
    const backward: Backward = { tag: "b", id: 2 };
    console.log(forward.tag, forward.id);
    console.log(backward.tag, backward.id);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "type-intersection-modifiers");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "a 1\nb 2\n");
    fs::remove_dir_all(root).unwrap();
}
