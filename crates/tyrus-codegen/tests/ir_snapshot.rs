use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

fn required_var(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must be set; run scripts/refactor-baseline"))
}

fn entry_points(list_path: &Path) -> Vec<PathBuf> {
    let listing = fs::read_to_string(list_path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", list_path.display()));
    listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(PathBuf::from)
        .collect()
}

fn snapshot_name(entry: &Path, taken: &mut BTreeMap<String, usize>) -> String {
    let stem = entry
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("program");
    let parent = entry
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("corpus");
    let base = if stem == "main" {
        format!("{parent}-{stem}")
    } else {
        stem.to_string()
    };
    let base = base.replace(['/', '.', ' '], "-");
    let count = taken.entry(base.clone()).or_insert(0);
    *count += 1;
    if *count == 1 {
        base
    } else {
        format!("{base}-{count}")
    }
}

#[test]
#[ignore = "baseline harness for the structural refactor; run by scripts/refactor-baseline"]
fn ir_snapshot() {
    let list_path = PathBuf::from(required_var("TYRUS_IR_SNAPSHOT_ENTRIES"));
    let out_dir = PathBuf::from(required_var("TYRUS_IR_SNAPSHOT_OUT"));
    fs::create_dir_all(&out_dir)
        .unwrap_or_else(|error| panic!("could not create {}: {error}", out_dir.display()));

    let entries = entry_points(&list_path);
    assert!(
        !entries.is_empty(),
        "corpus is empty: {}",
        list_path.display()
    );

    let mut taken = BTreeMap::new();
    let mut manifest = String::new();
    let mut failures = Vec::new();

    for entry in &entries {
        let name = snapshot_name(entry, &mut taken);
        let stem = out_dir.join(&name);
        let result = module_loader::load_program(entry, None)
            .and_then(|program| codegen::compile(&program, &name, &stem.to_string_lossy(), false));
        match result {
            Ok(()) => manifest.push_str(&format!("ok {name} {}\n", entry.display())),
            Err(error) => {
                let report = out_dir.join(format!("{name}.error.txt"));
                fs::write(&report, &error).unwrap_or_else(|write_error| {
                    panic!("could not write {}: {write_error}", report.display())
                });
                let first_line = error.lines().next().unwrap_or("unknown error").to_string();
                manifest.push_str(&format!("fail {name} {} {first_line}\n", entry.display()));
                failures.push(format!("{}: {first_line}", entry.display()));
            }
        }
    }

    let manifest_path = out_dir.join("manifest.txt");
    fs::write(&manifest_path, &manifest)
        .unwrap_or_else(|error| panic!("could not write {}: {error}", manifest_path.display()));

    let tolerate = std::env::var_os("TYRUS_IR_SNAPSHOT_ALLOW_FAILURES").is_some();
    if tolerate {
        eprintln!(
            "{} of {} corpus programs did not compile",
            failures.len(),
            entries.len()
        );
        return;
    }
    assert!(
        failures.is_empty(),
        "{} of {} corpus programs did not compile:\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n")
    );
}
