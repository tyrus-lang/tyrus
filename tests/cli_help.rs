use std::process::Command;

#[test]
fn help_commands_use_a_consistent_two_space_indent() {
    let output = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    for line in stdout.lines().filter(|line| line.contains("tyrus ")) {
        assert_eq!(
            line.chars()
                .take_while(|character| *character == ' ')
                .count(),
            2,
            "unexpected help indentation: {line:?}"
        );
        assert!(line.starts_with("  tyrus "));
    }
}
