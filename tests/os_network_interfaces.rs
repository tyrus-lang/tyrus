use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct NetworkInterfacesProject {
    root: PathBuf,
    name: String,
}

impl NetworkInterfacesProject {
    fn new() -> Self {
        let name = format!(
            "tyrus-os-network-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(r#"{{"project":{{"name":"{name}","version":"0.1.0"}},"dependencies":{{}}}}"#),
        )
        .unwrap();
        fs::write(
            root.join("src/main.tyr"),
            r#"import { NetworkInterface, OsError, networkInterfaces } from "tyrus:os";

const main = (): void => {
    const result: Result<NetworkInterface[], OsError> = networkInterfaces();
    match (result) {
        Ok(interfaces) => { console.log(`ok:${interfaces.length}`); }
        Err(error) => { console.log(error.code); }
    }
};
"#,
        )
        .unwrap();
        Self { root, name }
    }

    fn run(&self) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("run")
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for NetworkInterfacesProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn network_interface_enumeration_returns_a_result_with_the_native_array_layout() {
    let project = NetworkInterfacesProject::new();
    let output = project.run();
    assert!(
        output.status.success(),
        "tyrus run failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.starts_with("ok:") || stdout == "ERR_OS_NETWORK_INTERFACES_FAILED\n",
        "unexpected output: {stdout:?}"
    );
    assert!(
        project
            .root
            .join("target/debug")
            .join(format!("{}{}", project.name, std::env::consts::EXE_SUFFIX))
            .exists()
    );
}
