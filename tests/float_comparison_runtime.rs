//! Float comparison, where NaN is the case that separates the operators.
//!
//! NaN compares unequal to everything, itself included. That makes `===` false
//! for it, and it makes `!==` true, which is the only pair of answers under
//! which `!==` is the negation of `===`. Any other pair leaves a program with
//! no operator that can tell a NaN apart.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct FloatProject {
    root: PathBuf,
    name: String,
}

impl FloatProject {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("float-comparison-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "float comparison test" }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
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

    fn build_and_run(&self) -> String {
        let build = self.build();
        assert!(
            build.status.success(),
            "build failed: {}",
            String::from_utf8_lossy(&build.stderr)
        );
        let run = Command::new(self.root.join("target/debug").join(&self.name))
            .output()
            .unwrap();
        assert!(
            run.status.success(),
            "run failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );
        String::from_utf8_lossy(&run.stdout).into_owned()
    }
}

impl Drop for FloatProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// `!==` is the negation of `===`, NaN included.
///
/// It was not: `!==` lowered to an *ordered* not-equal, which is false when
/// either side is NaN, exactly like `===`. So `NaN === NaN` and `NaN !== NaN`
/// both answered `false`, two operators that are supposed to be opposites
/// agreeing, and no comparison in the language could see a NaN. Writing
/// `!(x === x)` worked, which is what makes the wrong answer easy to miss:
/// nothing fails, one spelling is simply wrong.
///
/// The ordinary values are here to keep the fix honest. An unordered comparison
/// changes nothing for them, and a test that only looked at NaN would pass for
/// an implementation that had broken every other float.
#[test]
fn float_inequality_is_the_negation_of_equality() {
    let project = FloatProject::new(
        r#"
function main(): void {
    const invalid: f64 = NaN;
    const one: f64 = 1.0;
    const two: f64 = 2.0;

    console.log(invalid === invalid);
    console.log(invalid !== invalid);
    console.log(invalid === one);
    console.log(invalid !== one);

    console.log(one === one);
    console.log(one !== one);
    console.log(one === two);
    console.log(one !== two);

    // Infinity is ordered, so it behaves like any other value.
    const far: f64 = Infinity;
    console.log(far === far);
    console.log(far !== far);
}
"#,
    );
    assert_eq!(
        project.build_and_run(),
        "false\ntrue\nfalse\ntrue\ntrue\nfalse\nfalse\ntrue\ntrue\nfalse\n"
    );
}

/// The same for `f32`, which is a different LLVM type and a separate chance to
/// get the predicate wrong.
#[test]
fn float32_inequality_sees_a_nan_too() {
    let project = FloatProject::new(
        r#"
function main(): void {
    const invalid: f32 = NaN;
    const value: f32 = 1.5;
    console.log(invalid !== invalid);
    console.log(invalid !== value);
    console.log(value !== value);
}
"#,
    );
    assert_eq!(project.build_and_run(), "true\ntrue\nfalse\n");
}
