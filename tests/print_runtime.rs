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
    "description": "print/printRaw runtime test"
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

/// `print` behaves exactly like `console.log`: space-joined arguments,
/// trailing `\n`, written to stdout.
#[test]
fn print_matches_console_log_on_stdout() {
    let root = project_root("print-matches-console-log");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    print("Ano:", 2026);
    console.log("Ano:", 2026);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "print-matches-console-log");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Ano: 2026\nAno: 2026\n"
    );
    assert!(output.stderr.is_empty());

    fs::remove_dir_all(root).unwrap();
}

/// `printRaw` never appends `\n`; consecutive calls concatenate on the same
/// line exactly as written.
#[test]
fn print_raw_never_appends_a_newline() {
    let root = project_root("print-raw-no-newline");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    printRaw("no-newline-");
    printRaw("still-one-line");
    print("");
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "print-raw-no-newline");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "no-newline-still-one-line\n"
    );

    fs::remove_dir_all(root).unwrap();
}

/// `console.warn`/`console.error` still target stderr; `print`/`printRaw`
/// never do, only `console.log`/`print`/`printRaw` write to stdout.
#[test]
fn print_targets_stdout_and_console_warn_error_target_stderr() {
    let root = project_root("print-stream-routing");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    print("out-line");
    console.warn("warn-line");
    console.error("error-line");
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "print-stream-routing");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "out-line\n");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "warn-line\nerror-line\n"
    );

    fs::remove_dir_all(root).unwrap();
}

/// `console.warn`/`console.error` colorize their whole line when the process
/// is told color is available (`FORCE_COLOR`), and `NO_COLOR` overrides that
/// back off -- the exact precedence `styleText` already documents. A piped
/// `Command` output is never a TTY, so absent either variable there must be
/// no ANSI bytes at all (already covered by the routing test above); `print`/
/// `console.log` on stdout never colorize regardless of these variables.
#[test]
fn console_warn_and_error_respect_force_color_and_no_color() {
    let root = project_root("print-color-env");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    print("plain");
    console.warn("warn-line");
    console.error("error-line");
};
"#,
    )
    .unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(build.status.success());
    let binary = root.join("target/debug").join("print-color-env");

    let forced = Command::new(&binary)
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(forced.stdout).unwrap(), "plain\n");
    assert_eq!(
        String::from_utf8(forced.stderr).unwrap(),
        "\x1b[33mwarn-line\x1b[0m\n\x1b[31merror-line\x1b[0m\n"
    );

    let no_color = Command::new(&binary)
        .env("FORCE_COLOR", "1")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(no_color.stderr).unwrap(),
        "warn-line\nerror-line\n",
        "NO_COLOR must win over FORCE_COLOR"
    );

    fs::remove_dir_all(root).unwrap();
}

/// `print`, `console`, and `assert` are reserved ambient builtins: a
/// user-defined function of the same name must fail the real `tyrus build`
/// CLI, not just the in-process parser unit tests.
#[test]
fn redeclaring_print_or_console_fails_the_build() {
    let root = project_root("print-redeclare-fails-build");
    fs::write(
        root.join("src/main.tyr"),
        r#"function print(label: string): void {
    console.log("shadowed:", label);
}

const main = (): void => { print("hello"); };
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        !build.status.success(),
        "redeclaring `print` must fail the build"
    );
    assert!(
        String::from_utf8_lossy(&build.stderr).contains("ambient builtin"),
        "expected the E0003 ambient-builtin message on stderr, got:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    fs::remove_dir_all(root).unwrap();
}

/// A `Command`'s captured stdout is always a pipe, never a TTY -- exactly
/// the case `printCR` must degrade for: no `\r`/`\x1B[K` bytes, just the
/// text written plainly, like `printRaw`. The ANSI-clearing behavior itself
/// is exercised manually (a real terminal is needed to observe it) and at
/// the codegen/IR level in `crates/tyrus-codegen/tests/unit/codegen.rs`.
#[test]
fn print_cr_writes_plain_text_when_stdout_is_not_a_terminal() {
    let root = project_root("print-cr-non-tty");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    printCR("=====>");
    printCR("=>");
    print("");
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "print-cr-non-tty");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout, "=====>=>\n",
        "piped stdout must never contain \\r or ANSI clear-line bytes"
    );
    assert!(!stdout.contains('\r'));
    assert!(!stdout.contains("\x1b["));

    fs::remove_dir_all(root).unwrap();
}

/// A union-typed value prints its variant wherever it comes from, and printing
/// it does not swallow what follows.
///
/// Three regressions in one program. The dispatch used to be reachable only for
/// a bare variable that had a stack slot, so a union arriving any other way,
/// an interface field, a call result, fell through to a match arm that emitted
/// *nothing at all* and printed an empty value with no diagnostic. And the
/// branch that did fire `return`ed instead of continuing the argument loop,
/// dropping every later argument and the trailing newline with them.
#[test]
fn a_union_prints_its_variant_from_every_position() {
    let root = project_root("print-union-positions");
    fs::write(
        root.join("src/main.tyr"),
        r#"type Status = "active" | "inactive" | "pending";

interface Row {
    id: i32;
    status: Status;
}

function show(s: Status): void {
    console.log("param:", s, "<end>");
}

function pick(flag: boolean): Status {
    if (flag) {
        return "active";
    }
    return "pending";
}

function main(): void {
    show("pending");
    const returned: Status = pick(true);
    console.log("returned:", returned, "<end>");
    const row: Row = { id: 1, status: "inactive" };
    console.log("field:", row.status, "<end>");
    console.log("last line");
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "print-union-positions");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "param: pending <end>\nreturned: active <end>\nfield: inactive <end>\nlast line\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A union interpolated in a template renders its variant, which used to be a
/// hard `error[codegen]: unsupported value in template interpolation`.
///
/// The `i32 | string` case is the one that matters for ownership: one arm is a
/// freshly allocated rendering of a number and the other is a pointer into
/// read-only memory, joined by a `phi`. What tells them apart afterwards is the
/// ownership bit inside the `string` value, not the branch that produced it.
#[test]
fn a_union_renders_inside_a_template_literal() {
    let root = project_root("print-union-template");
    fs::write(
        root.join("src/main.tyr"),
        r#"type Status = "active" | "inactive" | "pending";
type Mixed = i32 | string;

interface Row {
    id: i32;
    status: Status;
}

function pick(flag: boolean): Status {
    if (flag) {
        return "active";
    }
    return "pending";
}

function main(): void {
    const row: Row = { id: 1, status: "pending" };
    console.log(`field: ${row.status}!`);
    console.log(`call: ${pick(true)}!`);

    const number: Mixed = 42;
    const text: Mixed = "words";
    console.log(`mixed: ${number} and ${text}!`);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "print-union-template");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "field: pending!\ncall: active!\nmixed: 42 and words!\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// A union of literal types is represented by its tag alone, and the tag has to
/// name the right variant everywhere the value goes: through a field, through a
/// call, into a `match`, into a template.
///
/// The tag used to be wrong and invisible. `union_variant_for_expr` searched by
/// erased type, and every string-literal variant erases to `string`, so
/// `"pending"` in `"active" | "inactive" | "pending"` was tagged 0. Nothing
/// showed it while the payload carried the text and the tag only chose a
/// renderer, all three renderers being the one for `string`.
#[test]
fn a_literal_union_keeps_its_variant_through_every_hop() {
    let root = project_root("print-union-tag");
    fs::write(
        root.join("src/main.tyr"),
        r#"type Status = "active" | "inactive" | "pending";
type Level = 1 | 2 | 3;

interface Row {
    id: i32;
    status: Status;
}

function pick(index: i32): Status {
    if (index === 0) {
        return "active";
    }
    if (index === 1) {
        return "inactive";
    }
    return "pending";
}

function code(status: Status): i32 {
    match (status) {
        "active" => {
            return 1;
        }
        "inactive" => {
            return 2;
        }
        _ => {
            return 3;
        }
    }
}

function main(): void {
    for (let index: i32 = 0; index < 3; index += 1) {
        const status: Status = pick(index);
        console.log(status, code(status));
    }

    const row: Row = { id: 7, status: "pending" };
    console.log(row.status, code(row.status), row.id);
    console.log(`template ${row.status}`);

    const level: Level = 2;
    console.log(`level ${level}`);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "print-union-tag");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "active 1\ninactive 2\npending 3\npending 3 7\ntemplate pending\nlevel 2\n"
    );
    fs::remove_dir_all(root).unwrap();
}

/// `printError`/`printWarn` are ambient aliases for `console.error`/
/// `console.warn`: writing to stderr, never stdout, keeping `print` alone on
/// stdout exactly like the `console` family siblings do.
#[test]
fn print_error_and_print_warn_target_stderr() {
    let root = project_root("print-error-warn-stderr");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    print("out-line");
    printWarn("warn-line");
    printError("error-line");
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "print-error-warn-stderr");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "out-line\n");
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "warn-line\nerror-line\n"
    );

    fs::remove_dir_all(root).unwrap();
}

/// `printError`/`printWarn` colorize exactly like `console.error`/
/// `console.warn`: red and yellow ANSI codes on stderr under `FORCE_COLOR`,
/// with `NO_COLOR` winning back off, and never on stdout.
#[test]
fn print_error_and_print_warn_respect_force_color_and_no_color() {
    let root = project_root("print-error-warn-color-env");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    print("plain");
    printWarn("warn-line");
    printError("error-line");
};
"#,
    )
    .unwrap();
    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(build.status.success());
    let binary = root.join("target/debug").join("print-error-warn-color-env");

    let forced = Command::new(&binary)
        .env("FORCE_COLOR", "1")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert_eq!(String::from_utf8(forced.stdout).unwrap(), "plain\n");
    assert_eq!(
        String::from_utf8(forced.stderr).unwrap(),
        "\x1b[33mwarn-line\x1b[0m\n\x1b[31merror-line\x1b[0m\n"
    );

    let no_color = Command::new(&binary)
        .env("FORCE_COLOR", "1")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(no_color.stderr).unwrap(),
        "warn-line\nerror-line\n",
        "NO_COLOR must win over FORCE_COLOR for printWarn/printError too"
    );

    fs::remove_dir_all(root).unwrap();
}

/// `printError` and `printWarn` are reserved ambient builtins: redeclaring
/// either must fail the real `tyrus build` CLI with the same E0003 message
/// as `print`/`console`.
#[test]
fn redeclaring_print_error_or_print_warn_fails_the_build() {
    let root = project_root("print-error-warn-redeclare-fails-build");
    fs::write(
        root.join("src/main.tyr"),
        r#"function printWarn(label: string): void {
    console.warn("shadowed:", label);
}

const main = (): void => { printWarn("hello"); };
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        !build.status.success(),
        "redeclaring `printWarn` must fail the build"
    );
    assert!(
        String::from_utf8_lossy(&build.stderr).contains("ambient builtin"),
        "expected the E0003 ambient-builtin message on stderr, got:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    fs::remove_dir_all(root).unwrap();
}

/// A literal union is a value you can compare, branch on, hold in an array, and
/// call string methods on, five separate refusals from the type checker until
/// now, each of which made the type close to unusable.
///
/// Comparing it was `E0308: cannot compare ... with string`; an exhaustive
/// `match` over it still demanded a `_` (`E0310`); a ternary over two of its
/// variants was `E0308`; an array literal of its variants was `E0308`; and any
/// member on it was `E0609`/`E0599`.
#[test]
fn a_literal_union_is_usable_as_a_value() {
    let root = project_root("union-usable");
    fs::write(
        root.join("src/main.tyr"),
        r#"type Status = "active" | "inactive" | "pending";

function pick(index: i32): Status {
    if (index === 0) {
        return "active";
    }
    if (index === 1) {
        return "inactive";
    }
    return "pending";
}

// Exhaustive over the three variants, with no `_` arm.
function code(status: Status): i32 {
    match (status) {
        "active" => {
            return 1;
        }
        "inactive" => {
            return 2;
        }
        "pending" => {
            return 3;
        }
    }
}

function main(): void {
    for (let index: i32 = 0; index < 3; index += 1) {
        const status: Status = pick(index);
        console.log(status, code(status), status === "inactive", status.length);
    }

    const flag: boolean = false;
    const chosen: Status = flag ? "active" : pick(2);
    console.log(chosen, chosen.toUpperCase());

    const table: Status[] = ["pending", "active"];
    console.log(table[0], table[1], table.length);
}
"#,
    )
    .unwrap();
    let output = build_and_run(&root, "union-usable");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "active 1 false 6\n\
         inactive 2 true 8\n\
         pending 3 false 7\n\
         pending PENDING\n\
         pending active 2\n"
    );
    fs::remove_dir_all(root).unwrap();
}
