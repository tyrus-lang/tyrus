//! The invariant every validator rests on: for the same value and the same
//! rule, the check the compiler runs on a literal and the check it emits for a
//! dynamic value must reach the same verdict.
//!
//! It is what lets the type checker stay quiet about a literal it proved valid,
//! instead of demanding a `try`/`catch` around it. When the two disagree, a
//! program compiles clean and dies at runtime.
//!
//! They did disagree. The compile-time check reduced every number to `f64`,
//! whose 53-bit mantissa cannot hold an `i64` past 2^53, so `@min(9007199254740993)`
//! accepted `9007199254740992` (a value below the minimum) and the binary
//! aborted. Four defects of that family were found and fixed one by one; this
//! table is what makes the fifth one fail a test instead of reaching a user.

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

/// One rule, one target type, and two values that must land on opposite sides
/// of it. Both values are checked twice: once as a literal, by the compiler,
/// and once as the return of a function, by the code the compiler emits.
struct Case {
    decorator: &'static str,
    ty: &'static str,
    accepted: &'static str,
    rejected: &'static str,
}

const CASES: &[Case] = &[
    // ── Numeric ──────────────────────────────────────────────────────────────
    Case {
        decorator: "@min(18)",
        ty: "i32",
        accepted: "18",
        rejected: "17",
    },
    Case {
        decorator: "@max(10)",
        ty: "i32",
        accepted: "10",
        rejected: "11",
    },
    Case {
        decorator: "@range(1, 10)",
        ty: "i32",
        accepted: "5",
        rejected: "11",
    },
    Case {
        decorator: "@positive()",
        ty: "i32",
        accepted: "1",
        rejected: "0",
    },
    Case {
        decorator: "@negative()",
        ty: "i32",
        accepted: "-1",
        rejected: "0",
    },
    Case {
        decorator: "@nonNegative()",
        ty: "i32",
        accepted: "0",
        rejected: "-1",
    },
    Case {
        decorator: "@nonPositive()",
        ty: "i32",
        accepted: "0",
        rejected: "1",
    },
    Case {
        decorator: "@multipleOf(3)",
        ty: "i32",
        accepted: "9",
        rejected: "10",
    },
    Case {
        decorator: "@powerOfTwo()",
        ty: "i32",
        accepted: "8",
        rejected: "7",
    },
    Case {
        decorator: "@port()",
        ty: "i32",
        accepted: "8080",
        rejected: "80",
    },
    Case {
        decorator: "@statusCode()",
        ty: "i32",
        accepted: "200",
        rejected: "99",
    },
    Case {
        decorator: "@integer()",
        ty: "f64",
        accepted: "3.0",
        rejected: "3.5",
    },
    Case {
        decorator: "@safeInteger()",
        ty: "f64",
        accepted: "3.0",
        rejected: "3.5",
    },
    // The two that broke the invariant. `f64` rounds both of these to the same
    // number, so a check that goes through `f64` cannot tell them apart.
    Case {
        decorator: "@min(9007199254740993)",
        ty: "i64",
        accepted: "9007199254740993",
        rejected: "9007199254740992",
    },
    Case {
        decorator: "@multipleOf(2)",
        ty: "i64",
        accepted: "9007199254740992",
        rejected: "9007199254740993",
    },
    // ── String ───────────────────────────────────────────────────────────────
    Case {
        decorator: "@minLength(3)",
        ty: "string",
        accepted: "\"abc\"",
        rejected: "\"ab\"",
    },
    Case {
        decorator: "@maxLength(3)",
        ty: "string",
        accepted: "\"abc\"",
        rejected: "\"abcd\"",
    },
    Case {
        decorator: "@length(3)",
        ty: "string",
        accepted: "\"abc\"",
        rejected: "\"ab\"",
    },
    Case {
        decorator: "@notEmpty()",
        ty: "string",
        accepted: "\"a\"",
        rejected: "\"\"",
    },
    Case {
        decorator: "@notBlank()",
        ty: "string",
        accepted: "\"a\"",
        rejected: "\"   \"",
    },
    Case {
        decorator: "@isTrimmed()",
        ty: "string",
        accepted: "\"a\"",
        rejected: "\" a \"",
    },
    Case {
        decorator: "@isLowercase()",
        ty: "string",
        accepted: "\"abc\"",
        rejected: "\"Abc\"",
    },
    Case {
        decorator: "@isUppercase()",
        ty: "string",
        accepted: "\"ABC\"",
        rejected: "\"Abc\"",
    },
    Case {
        decorator: "@startsWith(\"user-\")",
        ty: "string",
        accepted: "\"user-ada\"",
        rejected: "\"admin\"",
    },
    Case {
        decorator: "@endsWith(\".md\")",
        ty: "string",
        accepted: "\"readme.md\"",
        rejected: "\"readme.txt\"",
    },
    Case {
        decorator: "@includes(\"ab\")",
        ty: "string",
        accepted: "\"xaby\"",
        rejected: "\"xy\"",
    },
    Case {
        decorator: "@regex(/^[a-z]+$/)",
        ty: "string",
        accepted: "\"abc\"",
        rejected: "\"A1\"",
    },
    // ── Format ───────────────────────────────────────────────────────────────
    Case {
        decorator: "@email()",
        ty: "string",
        accepted: "\"ada@example.com\"",
        rejected: "\"invalid\"",
    },
    Case {
        decorator: "@url()",
        ty: "string",
        accepted: "\"https://example.com\"",
        rejected: "\"not a url\"",
    },
    Case {
        decorator: "@uuid()",
        ty: "string",
        accepted: "\"550e8400-e29b-41d4-a716-446655440000\"",
        rejected: "\"not-a-uuid\"",
    },
    Case {
        decorator: "@ip()",
        ty: "string",
        accepted: "\"192.168.0.1\"",
        rejected: "\"999.1.1.1\"",
    },
    Case {
        decorator: "@ipv4()",
        ty: "string",
        accepted: "\"192.168.0.1\"",
        rejected: "\"::1\"",
    },
    Case {
        decorator: "@ipv6()",
        ty: "string",
        accepted: "\"::1\"",
        rejected: "\"192.168.0.1\"",
    },
    Case {
        decorator: "@e164()",
        ty: "string",
        accepted: "\"+5511999999999\"",
        rejected: "\"11999999999\"",
    },
    Case {
        decorator: "@cep()",
        ty: "string",
        accepted: "\"01310-100\"",
        rejected: "\"123\"",
    },
    Case {
        decorator: "@cpf()",
        ty: "string",
        accepted: "\"52998224725\"",
        rejected: "\"11111111111\"",
    },
    Case {
        decorator: "@cnpj()",
        ty: "string",
        accepted: "\"11222333000181\"",
        rejected: "\"11111111111111\"",
    },
    // ── Arrays ───────────────────────────────────────────────────────────────
    Case {
        decorator: "@minItems(2)",
        ty: "i32[]",
        accepted: "[1, 2]",
        rejected: "[1]",
    },
    Case {
        decorator: "@maxItems(2)",
        ty: "i32[]",
        accepted: "[1, 2]",
        rejected: "[1, 2, 3]",
    },
    Case {
        decorator: "@lengthItems(2)",
        ty: "i32[]",
        accepted: "[1, 2]",
        rejected: "[1]",
    },
    Case {
        decorator: "@notEmptyItems()",
        ty: "i32[]",
        accepted: "[1]",
        rejected: "[]",
    },
    Case {
        decorator: "@unique()",
        ty: "i32[]",
        accepted: "[1, 2]",
        rejected: "[1, 1]",
    },
    // A float is a number and not its bytes: `0.0` and `-0.0` are the same
    // number written with different bits, so an array holding both is not
    // unique, which is what `0.0 === -0.0` already said.
    Case {
        decorator: "@unique()",
        ty: "f64[]",
        accepted: "[1.5, 2.5]",
        rejected: "[0.0, -0.0]",
    },
    // ── Unicode ──────────────────────────────────────────────────────────────
    //
    // The string rules are the ones a sampled table is weakest at: an ASCII-only
    // corpus agrees under almost any implementation, so it would not notice one
    // side counting bytes or testing case character by character. These are the
    // values where a wrong implementation gives a different answer.
    //
    // `"ção"` is three characters and five bytes: a length measured in bytes
    // would make `@maxLength(3)` reject it.
    Case {
        decorator: "@maxLength(3)",
        ty: "string",
        accepted: "\"ção\"",
        rejected: "\"ções\"",
    },
    Case {
        decorator: "@length(3)",
        ty: "string",
        accepted: "\"ção\"",
        rejected: "\"ções\"",
    },
    // `ß` is lowercase, and it uppercases to `SS`, so it is not uppercase,
    // although it has no uppercase form of its own.
    Case {
        decorator: "@isLowercase()",
        ty: "string",
        accepted: "\"ß\"",
        rejected: "\"İ\"",
    },
    // The Turkish dotted `İ` is uppercase; lowercasing it yields two code
    // points, which is why the rule is a round trip and not a per-character test.
    Case {
        decorator: "@isUppercase()",
        ty: "string",
        accepted: "\"İ\"",
        rejected: "\"ß\"",
    },
    // A non-breaking space is whitespace, so a string made of one is blank and
    // a string ending in one is not trimmed.
    Case {
        decorator: "@notBlank()",
        ty: "string",
        accepted: "\"á\"",
        rejected: "\"\u{00a0}\"",
    },
    Case {
        decorator: "@isTrimmed()",
        ty: "string",
        accepted: "\"á\"",
        rejected: "\"á\u{00a0}\"",
    },
    // ── Values ───────────────────────────────────────────────────────────────
    Case {
        decorator: "@in([1, 2])",
        ty: "i32",
        accepted: "1",
        rejected: "3",
    },
    Case {
        decorator: "@notIn([1, 2])",
        ty: "i32",
        accepted: "3",
        rejected: "1",
    },
    Case {
        decorator: "@equal(5)",
        ty: "i32",
        accepted: "5",
        rejected: "6",
    },
    Case {
        decorator: "@notEqual(5)",
        ty: "i32",
        accepted: "6",
        rejected: "5",
    },
];

/// What the compiler says about the value written as a literal.
fn accepts_literal(case: &Case, value: &str) -> bool {
    let source = format!(
        "interface Subject {{ {} value: {}; }}\nconst subject: Subject = {{ value: {} }};\n",
        case.decorator, case.ty, value
    );
    let analysis = tyrus_core::analysis::analyze(&source);
    let failures: Vec<_> = analysis
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code != "unused-variable")
        .collect();
    assert!(
        failures.iter().all(|diagnostic| diagnostic.code == "E3165"),
        "`{}` on `{}` with `{}` raised something other than a validation failure: {:?}",
        case.decorator,
        case.ty,
        value,
        failures
    );
    failures.is_empty()
}

/// What the check the compiler emits says about the same value, reached through
/// a function so that it is not a literal any more.
fn accepts_at_runtime(cases: &[Case]) -> Vec<(bool, bool)> {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-validator-agreement-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "validator-agreement-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Static and runtime validator agreement"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();

    let mut declarations = String::new();
    let mut body = String::new();
    for (index, case) in cases.iter().enumerate() {
        for (half, value) in [case.accepted, case.rejected].iter().enumerate() {
            let id = index * 2 + half;
            declarations.push_str(&format!(
                "interface Subject{id} {{\n    {}\n    value: {};\n}}\n\nconst read{id} = (): {} => {};\n\n",
                case.decorator, case.ty, case.ty, value
            ));
            // The value arrives from a function, so the type checker cannot fold
            // it: this exercises the emitted check, not the compile-time one.
            body.push_str(&format!(
                "    try {{\n        const subject{id}: Subject{id} = {{ value: read{id}() }};\n        console.log(\"{id}:ok\", subject{id}.value);\n    }} catch (_error: ValidationError) {{\n        console.log(\"{id}:fail\");\n    }}\n"
            ));
        }
    }
    fs::write(
        root.join("src/main.tyr"),
        format!("{declarations}const main = (): void => {{\n{body}}};\n"),
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/validator-agreement-test"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the program aborted: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();

    let verdicts: Vec<bool> = stdout
        .lines()
        .map(|line| {
            let (_, verdict) = line
                .split_once(':')
                .expect("every line carries its case id");
            !verdict.starts_with("fail")
        })
        .collect();
    assert_eq!(
        verdicts.len(),
        cases.len() * 2,
        "every case must report exactly one verdict"
    );

    fs::remove_dir_all(root).unwrap();
    verdicts.chunks(2).map(|pair| (pair[0], pair[1])).collect()
}

/// `@notNull`, `@required` and `@optional` are the three validators the table
/// above cannot express: they test presence itself, so their two sides are an
/// optional carrying a value and an optional carrying none, not two values.
#[test]
fn the_presence_validators_agree_about_an_absent_optional() {
    let suffix = format!(
        "{}-{}-{}",
        std::process::id(),
        PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let root = std::env::temp_dir().join(format!("tyrus-validator-presence-{suffix}"));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{
  "project": {
    "name": "validator-presence-test",
    "version": "0.1.0",
    "authors": [],
    "description": "Presence validators, statically and at runtime"
  },
  "dependencies": {}
}
"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"interface Required {
    @required()
    email: string?;
}

interface NotNull {
    @notNull()
    email: string?;
}

interface Optional {
    @optional()
    email: string?;
}

const readPresent = (): string? => {
    return "ada@example.com";
};

const readAbsent = (): string? => {
    return;
};

const main = (): void => {
    try {
        const present: Required = { email: readPresent() };
        console.log("required-present:ok", present.email ?? "unreachable");
    } catch (_error: ValidationError) {
        console.log("required-present:fail");
    }

    try {
        const absent: Required = { email: readAbsent() };
        console.log("required-absent:ok", absent.email ?? "absent");
    } catch (_error: ValidationError) {
        console.log("required-absent:fail");
    }

    try {
        const absent: NotNull = { email: readAbsent() };
        console.log("notnull-absent:ok", absent.email ?? "absent");
    } catch (_error: ValidationError) {
        console.log("notnull-absent:fail");
    }

    // `@optional` marks the absence as allowed, so it never fails.
    try {
        const absent: Optional = { email: readAbsent() };
        console.log("optional-absent:ok", absent.email ?? "absent");
    } catch (_error: ValidationError) {
        console.log("optional-absent:fail");
    }
};
"#,
    )
    .unwrap();

    let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let output = Command::new(root.join("target/debug/validator-presence-test"))
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    fs::remove_dir_all(root).unwrap();

    // `@required` and `@notNull` test presence, so an absent optional fails
    // them and a present one satisfies them. `@optional` allows the absence.
    assert_eq!(
        stdout,
        "required-present:ok ada@example.com\nrequired-absent:fail\nnotnull-absent:fail\noptional-absent:ok absent\n"
    );
}

#[test]
fn the_literal_check_and_the_emitted_check_agree_on_every_validator() {
    let runtime = accepts_at_runtime(CASES);

    for (case, (accepted_at_runtime, rejected_at_runtime)) in CASES.iter().zip(runtime) {
        let accepted_as_literal = accepts_literal(case, case.accepted);
        let rejected_as_literal = accepts_literal(case, case.rejected);

        // The value that satisfies the rule passes both checks.
        assert!(
            accepted_as_literal,
            "`{}` on `{}` rejected `{}` as a literal, but the rule accepts it",
            case.decorator, case.ty, case.accepted
        );
        assert!(
            accepted_at_runtime,
            "`{}` on `{}` rejected `{}` at runtime, but the rule accepts it",
            case.decorator, case.ty, case.accepted
        );

        // The value that breaks it fails both. A rule that only fails as a
        // literal blocks valid code; one that only fails at runtime lets a
        // program compile clean and then abort, that is the unsound half.
        assert!(
            !rejected_as_literal,
            "`{}` on `{}` accepted `{}` as a literal, but the rule rejects it: a program that compiles clean will abort on the check emitted for it",
            case.decorator, case.ty, case.rejected
        );
        assert!(
            !rejected_at_runtime,
            "`{}` on `{}` emitted no check that catches `{}`: the rule is silently absent at runtime",
            case.decorator, case.ty, case.rejected
        );
    }
}
