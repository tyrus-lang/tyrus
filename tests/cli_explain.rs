use std::process::Command;

fn tyrus(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn explain_prints_public_diagnostic_metadata() {
    let output = tyrus(&["explain", "E0308"]);

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "E0308: Mismatched types\n  Category: Type checking\n  Documentation: https://tyruslang.org/language/errors#error-codes\n"
    );
}

#[test]
fn explain_rejects_unknown_or_missing_codes() {
    let unknown = tyrus(&["explain", "E9999"]);
    assert!(!unknown.status.success());
    assert_eq!(
        String::from_utf8(unknown.stderr).unwrap(),
        "error: unknown public compiler diagnostic `E9999`\n"
    );

    let missing = tyrus(&["explain"]);
    assert!(!missing.status.success());
    assert_eq!(
        String::from_utf8(missing.stderr).unwrap(),
        "error: missing error code\nUsage: tyrus explain <error-code>\n"
    );
}
