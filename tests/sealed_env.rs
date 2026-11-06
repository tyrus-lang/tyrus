//! Sealed env, end to end: build a project with `security.envSealed`, then run
//! the binary it produces.
//!
//! Sealing moves the values out of the source tree and into an encrypted
//! artifact next to the binary, decrypted at startup with a key kept in the
//! user's configuration directory. Both the compiler and the runtime resolve
//! that key through `directories::BaseDirs`, which honours `XDG_CONFIG_HOME`, so
//! every test here points it at its own temporary directory: the suite must
//! never read or write the developer's real key store.

use std::fs;
use std::path::{Path, PathBuf};
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

/// A value that must never appear in a diagnostic, a binary, or an artifact.
const SECRET: &str = "sk-live-9f3a7c21-do-not-leak";

struct SealedProject {
    root: PathBuf,
    config_home: PathBuf,
    name: String,
}

impl SealedProject {
    fn new(label: &str, env_file: &str, main: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("sealed-{label}-{suffix}");
        let root = std::env::temp_dir().join(&name);
        let config_home = root.join("config-home");
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&config_home).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Sealed env test"
  }},
  "security": {{
    "envFile": ".env",
    "envSealed": true
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), env_file).unwrap();
        fs::write(root.join("src/main.tyr"), main).unwrap();
        Self {
            root,
            config_home,
            name,
        }
    }

    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .env("XDG_CONFIG_HOME", &self.config_home)
            .output()
            .unwrap()
    }

    fn binary(&self) -> PathBuf {
        self.root.join("target/debug").join(&self.name)
    }

    fn sealed_artifact(&self) -> PathBuf {
        self.root.join("target/debug").join(".env.sealed")
    }

    /// Runs the built binary with `extra` added to its environment.
    fn run(&self, extra: &[(&str, &str)]) -> Output {
        let mut command = Command::new(self.binary());
        command.env("XDG_CONFIG_HOME", &self.config_home);
        for (key, value) in extra {
            command.env(key, value);
        }
        command.output().unwrap()
    }
}

impl Drop for SealedProject {
    fn drop(&mut self) {
        // The key lives under the project's own config home, so removing the
        // project removes the key with it.
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn contains_secret(bytes: &[u8]) -> bool {
    bytes
        .windows(SECRET.len())
        .any(|window| window == SECRET.as_bytes())
}

fn assert_no_secret(label: &str, path: &Path) {
    let bytes = fs::read(path).unwrap();
    assert!(
        !contains_secret(&bytes),
        "`{label}` ({}) contains the secret verbatim",
        path.display()
    );
}

/// The point of sealing: the secret leaves the source tree, and the binary the
/// build produces does not carry it. The value reaches the program only by
/// decrypting the artifact at startup, so neither the executable nor the sealed
/// file may hold the plaintext.
#[test]
fn a_sealed_build_embeds_the_secret_in_neither_the_binary_nor_the_artifact() {
    let project = SealedProject::new(
        "no-embed",
        &format!(
            "@type(string)\n@minLength(8)\nAPI_TOKEN={SECRET}\n\n@type(u32)\n@port()\nPORT=8080\n"
        ),
        r#"const main = (): void => {
    // The port, never the token: what the program prints is its own choice,
    // and this one deliberately keeps the secret out of stdout.
    console.log(process.env.PORT);
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    assert_no_secret("binary", &project.binary());
    assert_no_secret("sealed artifact", &project.sealed_artifact());

    let output = project.run(&[]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "8080\n");
    assert!(!contains_secret(&output.stderr));
}

/// A sealed value is the mandatory source of truth. A process environment
/// variable of the same name must not override it, or sealing would be a
/// suggestion rather than a guarantee: anyone able to set an env var could
/// swap a value the build had validated for one it never saw.
#[test]
fn a_process_environment_variable_does_not_override_a_sealed_value() {
    let project = SealedProject::new(
        "no-override",
        "@type(string)\n@in([\"development\", \"production\"])\nTYRUS_ENV=production\n",
        r#"const main = (): void => {
    console.log(process.env.TYRUS_ENV);
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    // The same variable, set in the environment the binary runs in, with a value
    // the sealed env does not hold.
    let output = project.run(&[("TYRUS_ENV", "development")]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "production\n",
        "a process environment variable overrode the sealed value"
    );
}

/// Validator decorations run against a sealed env exactly as they do against a
/// plain one, and the diagnostic that reports a failure must name the variable
/// and the rule without ever echoing the value. A failing build writes straight
/// into a CI log, and the code frame prints the line it points at, so redaction
/// has to reach the frame and not only the message.
#[test]
fn a_failing_validator_on_a_sealed_env_never_prints_the_value() {
    let project = SealedProject::new(
        "redaction",
        // 27 characters against a rule that demands 64: rejected at compile time,
        // because the value is known then.
        &format!("@type(string)\n@minLength(64)\nAPI_TOKEN={SECRET}\n"),
        r#"const main = (): void => {
    console.log(process.env.API_TOKEN);
};
"#,
    );

    let build = project.build();
    assert!(
        !build.status.success(),
        "a value that breaks `@minLength(64)` must not build"
    );

    let stdout = String::from_utf8_lossy(&build.stdout);
    let stderr = String::from_utf8_lossy(&build.stderr);
    let reported = format!("{stdout}{stderr}");

    assert!(
        reported.contains("API_TOKEN"),
        "the diagnostic must name the variable that failed:\n{reported}"
    );
    assert!(
        reported.contains("minLength"),
        "the diagnostic must name the rule that failed:\n{reported}"
    );
    assert!(
        !reported.contains(SECRET),
        "the diagnostic leaked the secret value:\n{reported}"
    );
    // Not even a prefix of it: a partial echo still publishes the beginning of a
    // key, and its length.
    assert!(
        !reported.contains("sk-live"),
        "the diagnostic leaked part of the secret value:\n{reported}"
    );
}

/// The binary refuses a sealed artifact it was not built for. The fingerprint
/// binds the executable to the key that sealed the file, so rotating the key,
/// which reseals the env under a new one, must make the previously built binary
/// abort rather than run against an artifact it cannot vouch for.
#[test]
fn a_binary_rejects_a_sealed_artifact_sealed_under_another_key() {
    let project = SealedProject::new(
        "fingerprint",
        "@type(u32)\n@port()\nPORT=8080\n",
        r#"const main = (): void => {
    console.log(process.env.PORT);
};
"#,
    );

    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\nstderr:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let binary = project.root.join("first-binary");
    fs::copy(project.binary(), &binary).unwrap();

    // A new key pair reseals the env under a fingerprint the first binary does
    // not carry.
    let rotate = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("build")
        .arg("--rotate-seal-key")
        .current_dir(&project.root)
        .env("XDG_CONFIG_HOME", &project.config_home)
        .output()
        .unwrap();
    assert!(
        rotate.status.success(),
        "rotation failed:\nstderr:\n{}",
        String::from_utf8_lossy(&rotate.stderr)
    );

    let output = Command::new(&binary)
        .env("XDG_CONFIG_HOME", &project.config_home)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "the binary ran against an artifact sealed under a different key"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("fingerprint"),
        "the abort must say the fingerprint did not match, found:\n{stderr}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).is_empty(),
        "a runtime abort must not write to stdout"
    );
}
