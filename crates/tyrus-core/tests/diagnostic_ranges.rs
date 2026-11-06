use std::fmt::Write as _;
use std::path::PathBuf;

use tyrus_core::analysis::{DiagnosticSeverity, analyze};

const SNAPSHOT: &str = "tests/snapshots/diagnostic-ranges.md";

#[allow(clippy::too_many_lines)]
fn cases() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "E3108: an inline handler that answers by returning",
            r#"import { createRouter, HttpReply, HttpRouter, HttpStatus } from "tyrus:http";

interface Health {
    ok: boolean;
}

const main = (): void => {
    const app: HttpRouter = createRouter();
    app.get("/health", {
        response: { 200: Health },
        handler: (request): Health => {
            return { ok: true };
        },
    });
};
"#,
        ),
        (
            "E3011: the offending call comes before a handled one",
            r#"const readAge = (): i32 => 16;

const acceptAge = (@min(18) age: i32): void => {
    console.log(age);
};

const main = (): void => {
    acceptAge(readAge());

    try {
        acceptAge(readAge());
    } catch (error: ValidationError) {
        console.log(error.message);
    }
};
"#,
        ),
        (
            "E0412: a catch annotation names a native interface that was not imported",
            r#"const main = (): void => {
    try {
        console.log(1);
    } catch (error: EncodingError) {
        console.log(error.message);
    }
    console.log("EncodingError");
};
"#,
        ),
        (
            "E3165: an invalid field before a valid one",
            r#"interface Filter {
    @min(5)
    pageSize: i32;
}

const main = (): void => {
    const bad: Filter = { pageSize: 3 };
    const good: Filter = { pageSize: 9 };
    console.log(bad.pageSize);
    console.log(good.pageSize);
};
"#,
        ),
        (
            "E3160",
            r#"interface Account {
    @doesNotExist()
    name: string;
}

const main = (): void => {
    const account: Account = { name: "ada" };
    console.log(account.name);
};
"#,
        ),
        (
            "E3161",
            r#"interface Account {
    @min(3)
    name: string;
}

const main = (): void => {
    const account: Account = { name: "ada" };
    console.log(account.name);
};
"#,
        ),
        (
            "E0308: type mismatch on an initializer, with a later use",
            r#"const main = (): void => {
    const total: i32 = "not a number";
    console.log(total);
};
"#,
        ),
        (
            "E0425: unresolved name",
            r#"const main = (): void => {
    console.log(missingName);
};
"#,
        ),
        (
            "E0384: assignment to a const, with a later read",
            r#"const main = (): void => {
    const limit: i32 = 1;
    limit = 2;
    console.log(limit);
};
"#,
        ),
        (
            "E0599: unknown method, called before a valid method on the same value",
            r#"const main = (): void => {
    const name: string = "ada";
    console.log(name.doesNotExist());
    console.log(name.length);
};
"#,
        ),
        (
            "E0609: unknown field, read before a valid field on the same value",
            r#"interface Point {
    x: i32;
}

const main = (): void => {
    const point: Point = { x: 1 };
    console.log(point.y);
    console.log(point.x);
};
"#,
        ),
        (
            "E0061: wrong arity, on the first of two calls",
            r#"const add = (a: i32, b: i32): i32 => a + b;

const main = (): void => {
    console.log(add(1));
    console.log(add(1, 2));
};
"#,
        ),
        (
            "E0716: force unwrap of an optional, with a later guarded use",
            r#"const main = (): void => {
    const maybe: i32? = 1;
    console.log(maybe!);
    console.log(maybe ?? 0);
};
"#,
        ),
        (
            "E0606: constant index out of bounds, before an in-bounds index",
            r#"const main = (): void => {
    const xs: i32[] = [1, 2, 3];
    console.log(xs[9]);
    console.log(xs[1]);
};
"#,
        ),
        (
            "E0606: constant index after an unconditional array reassignment",
            r#"const main = (): void => {
    const index = 1;
    let array = [1, 2, 3, 4, 5];
    if (index === 1) {
        array = [];
    }
    console.log(array[2]);
};
"#,
        ),
        (
            "E0428: duplicate declaration",
            r#"const main = (): void => {
    const value: i32 = 1;
    const value: i32 = 2;
    console.log(value);
};
"#,
        ),
        (
            "E0268: break outside a loop or switch",
            r#"const main = (): void => {
    break;
};
"#,
        ),
        (
            "E0277: unsupported operand types",
            r#"const main = (): void => {
    const flag: boolean = true;
    const total: i32 = flag - 1;
    console.log(total);
};
"#,
        ),
        (
            "E3000",
            r#"const load = async (): Promise<i32> => 1;

const main = (): void => {
    const value: i32 = await load();
    console.log(value);
};
"#,
        ),
        (
            "E3080",
            r#"const main = (): void => {
    const value: i32? = null;
        console.log(value ?? 0);
};
"#,
        ),
        (
            "E0308: numeric mismatch on a compound assignment, with a later read",
            r#"const main = (): void => {
    let sum = 0;
    const values: i32[] = [0, 1, 2, 3];
    for (let i = 0; i < values.length; i += 1) {
        sum += values[i];
        sum += i;
    }
    console.log(sum);
};
"#,
        ),
        (
            "E0794: a composite value stored in unknown, read after the declaration",
            r#"const main = (): void => {
    const scores: unknown = [90, 85];
    console.log(scores);
};
"#,
        ),
        (
            "E0795: typeof compared with a capitalized category",
            r#"const main = (): void => {
    const value: unknown = "a";
    if (typeof value === "String") {
        console.log(value);
    }
};
"#,
        ),
        (
            "E0795: a category from another language on the left of the comparison",
            r#"const main = (): void => {
    const value: string | i32 = 1;
    if ("number" !== typeof value) {
        console.log(value);
    }
};
"#,
        ),
        (
            "E3081: a narrowed optional field bound with its narrowed type",
            r#"interface Inner {
    tag?: string;
}

interface Outer {
    inner: Inner;
}

const main = (): void => {
    const outer: Outer = { inner: { tag: "ada" } };
    if (outer.inner.tag) {
        const taken: string = outer.inner.tag;
        console.log(taken);
    }
};
"#,
        ),
        (
            "E0382: a field moved out and read again",
            r#"interface Basket {
    items: string[];
}

const main = (): void => {
    const basket: Basket = { items: ["a"] };
    const taken: string[] = basket.items;
    console.log(taken.length);
    console.log(basket.items.length);
};
"#,
        ),
        (
            "E0308: a union return, wrong first and right afterwards",
            r#"type Answer = string | i32;

const pick = (flag: boolean): Answer => {
    if (flag) {
        return true;
    }
    if (!flag) {
        return "no";
    }
    return 7;
};

const main = (): void => {
    console.log(pick(true));
};
"#,
        ),
        (
            "E0308: a union of interfaces returning an unrelated object",
            r#"interface Ok {
    value: i32;
}

interface Fail {
    reason: string;
}

type Outcome = Ok | Fail;

const bad = (flag: boolean): Outcome => {
    if (flag) {
        return { value: 1 };
    }
    return { unexpected: true };
};

const main = (): void => {
    console.log(bad(true));
};
"#,
        ),
        (
            "E3292: the offending match arm, with a well-formed arm after it",
            r#"enum Command {
    Move(i32, i32),
    Quit,
}

const main = (): void => {
    const command: Command = Command.Move(10, 20);
    match (command) {
        Command.Move(position) => { console.log(1); }
        Command.Quit => { console.log(0); }
    }
};
"#,
        ),
    ]
}

fn code_frame(source: &str, range: &std::ops::Range<usize>) -> String {
    let start = range.start.min(source.len());
    let end = range.end.min(source.len()).max(start);
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[start..]
        .find('\n')
        .map_or(source.len(), |index| start + index);
    let line = &source[line_start..line_end];
    let line_number = source[..start].matches('\n').count() + 1;
    let column = start - line_start + 1;
    let width = end.saturating_sub(start).max(1);

    let mut frame = String::new();
    let _ = writeln!(frame, "  at {line_number}:{column}");
    let _ = writeln!(frame, "      {line}");
    let _ = writeln!(
        frame,
        "      {}{}",
        " ".repeat(start - line_start),
        "^".repeat(width)
    );
    frame
}

fn render() -> String {
    let mut out = String::new();
    out.push_str("# Diagnostic range snapshot\n\n");
    out.push_str("Generated by `cargo test -p tyrus-core --test diagnostic_ranges`. Do not edit by hand; regenerate with `TYRUS_UPDATE_SNAPSHOT=1`.\n\n");
    out.push_str("Each case shows the program, then every diagnostic it publishes with the caret the editor and the terminal draw. A caret that moves in a diff is an anchoring change, intended or not.\n");

    for (name, source) in cases() {
        let _ = write!(out, "\n## {name}\n\n```tyrus\n{source}```\n\n");
        let diagnostics = analyze(source).diagnostics;
        if diagnostics.is_empty() {
            out.push_str("_no diagnostics_\n");
            continue;
        }
        for diagnostic in diagnostics {
            let severity = match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
            };
            let _ = write!(
                out,
                "```text\n{severity}[{}]: {}\n{}",
                diagnostic.code,
                diagnostic.message,
                code_frame(source, &diagnostic.range)
            );
            for related in &diagnostic.related {
                let _ = write!(
                    out,
                    "  related: {}\n{}",
                    related.message,
                    code_frame(source, &related.range)
                );
            }
            for help in &diagnostic.help {
                let _ = writeln!(out, "  help: {help}");
            }
            out.push_str("```\n");
        }
    }
    out
}

fn snapshot_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT)
}

#[test]
fn diagnostic_ranges_match_the_snapshot() {
    let rendered = render();
    let path = snapshot_path();

    if std::env::var_os("TYRUS_UPDATE_SNAPSHOT").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &rendered).unwrap();
        return;
    }

    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing snapshot at {}; regenerate with `TYRUS_UPDATE_SNAPSHOT=1 cargo test -p tyrus-core --test diagnostic_ranges`",
            path.display()
        )
    });
    if rendered != expected {
        let mut report = String::from(
            "the published diagnostic ranges changed; review the diff and, if intended, regenerate with `TYRUS_UPDATE_SNAPSHOT=1 cargo test -p tyrus-core --test diagnostic_ranges`\n\n",
        );
        for (line, (actual, expected)) in rendered.lines().zip(expected.lines()).enumerate() {
            if actual != expected {
                let _ = write!(
                    report,
                    "first difference at line {}\n  expected: {expected}\n  actual:   {actual}\n",
                    line + 1
                );
                break;
            }
        }
        panic!("{report}");
    }
}
