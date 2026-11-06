//! Interfaces reached through the fields of an imported interface.
//!
//! `person.address.city` failed with `E0609` ("no field `city` on type
//! `Address`") unless `Address` was imported too, although the program never
//! writes that name. The `api` scaffold and the named handler docs imported
//! every type along the way to get past it.

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

struct ModuleProject {
    root: PathBuf,
    name: String,
}

impl ModuleProject {
    fn new(main: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("module-imports-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src/schemas")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Module imports integration test" }},
  "security": {{ "envFile": ".env", "envSealed": false }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
        fs::write(
            root.join("src/schemas/person.tyr"),
            r#"export interface Country {
    code: string;
}

export interface Address {
    city: string;
    country: Country;
}

export interface Person {
    name: string;
    address: Address;
}

export const makePerson = (): Person => {
    return { name: "Ada", address: { city: "London", country: { code: "GB" } } };
};
"#,
        )
        .unwrap();
        fs::write(root.join("src/main.tyr"), main).unwrap();
        Self { root, name }
    }

    fn build(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }

    fn run(&self) -> Output {
        Command::new(self.root.join("target/debug").join(&self.name))
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for ModuleProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn fields_of_an_imported_interface_resolve_through_types_it_does_not_import() {
    let project = ModuleProject::new(
        r#"import { Person, makePerson } from "./schemas/person";

const main = (): void => {
    const person: Person = makePerson();
    console.log(person.address.city);
    console.log(person.address.country.code);
};
"#,
    );
    let build = project.build();
    assert!(
        build.status.success(),
        "build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let output = project.run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "London\nGB\n");
}

#[test]
fn naming_a_reachable_interface_still_needs_its_import() {
    let project = ModuleProject::new(
        r#"import { Person, makePerson } from "./schemas/person";

const main = (): void => {
    const person: Person = makePerson();
    const address: Address = person.address;
    console.log(address.city);
};
"#,
    );
    let build = project.build();
    assert!(!build.status.success(), "the build should have failed");
    let stderr = String::from_utf8_lossy(&build.stderr);
    assert!(
        stderr.contains("E0412")
            && stderr.contains("cannot find type `Address`")
            && stderr.contains("add `Address` to the import"),
        "unexpected diagnostic: {stderr}"
    );
}
