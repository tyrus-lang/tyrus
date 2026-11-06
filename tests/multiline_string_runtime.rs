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
    "description": "smart dedent runtime test"
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

/// `"""` dedents a SQL query and an HTML template against their closing
/// delimiter's indentation, with zero runtime cost: both come out of the
/// compiler as plain string literals.
#[test]
fn triple_quote_dedents_sql_and_html_against_closing_indent() {
    let root = project_root("multiline-sql-html");
    fs::write(
        root.join("src/main.tyr"),
        r#"function buildQuery(): string {
    const query = """
        SELECT id, nome
        FROM usuarios
        WHERE ativo = true
        """;
    return query;
}

function buildHtml(): string {
    const html = """
        <html>
          <body>Hello</body>
        </html>
        """;
    return html;
}

const main = (): void => {
    console.log(buildQuery());
    console.log(buildHtml());
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "multiline-sql-html");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "SELECT id, nome\nFROM usuarios\nWHERE ativo = true\n<html>\n  <body>Hello</body>\n</html>\n"
    );

    fs::remove_dir_all(root).unwrap();
}

/// The dedent margin is read from the line the closing `"""` sits on, not
/// from whatever statement happens to follow the string in the file.
#[test]
fn triple_quote_dedent_ignores_differently_indented_trailing_code() {
    let root = project_root("multiline-trailing-code");
    fs::write(
        root.join("src/main.tyr"),
        r#"const main = (): void => {
    const query = """
        SELECT 1
        """;
    console.log(query);
};
"#,
    )
    .unwrap();

    let output = build_and_run(&root, "multiline-trailing-code");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "SELECT 1\n");

    fs::remove_dir_all(root).unwrap();
}
