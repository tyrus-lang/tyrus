//! Archives cross the boundary in both directions with the tools people
//! actually use: GNU tar, bsdtar, Info-ZIP and Python's `zipfile`. A tool
//! missing from the machine skips its half of a test instead of failing it.

use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("tyrus-archive-interop-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn text(&self, relative: &str) -> String {
        self.path(relative).to_string_lossy().into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A name past the 100-byte ustar field, so every tool has to use its
/// long-name extension (GNU `L` headers, PAX `path`, ZIP has no limit).
fn long_name() -> String {
    format!(
        "{}/deep.txt",
        "directory-with-a-long-name/"
            .repeat(4)
            .trim_end_matches('/')
    )
}

fn sample(scratch: &Scratch) -> Vec<(String, Vec<u8>)> {
    let files = vec![
        ("readme.txt".to_string(), b"hello interop\n".to_vec()),
        ("data/zeros.bin".to_string(), vec![0u8; 256 * 1024]),
        (
            "data/unicode-é.txt".to_string(),
            "ação\n".as_bytes().to_vec(),
        ),
        (long_name(), b"long\n".to_vec()),
    ];
    for (name, contents) in &files {
        let path = scratch.path("src").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    fs::create_dir_all(scratch.path("src/empty")).unwrap();
    files
}

fn assert_same_tree(root: &Path, files: &[(String, Vec<u8>)]) {
    for (name, contents) in files {
        assert_eq!(
            fs::read(root.join(name)).unwrap_or_else(|error| panic!("{name}: {error}")),
            *contents,
            "{name}"
        );
    }
    assert!(root.join("empty").is_dir(), "empty directory missing");
}

fn available(tool: &str) -> bool {
    let probe = if tool == "unzip" { "-v" } else { "--version" };
    Command::new(tool)
        .arg(probe)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?} failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn archives_written_by_system_tools_extract_identically() {
    let scratch = Scratch::new();
    let files = sample(&scratch);
    let src = scratch.path("src");
    let mut archives = Vec::new();
    if available("tar") {
        for (format, name) in [("gnu", "gnu.tar"), ("posix", "posix.tar")] {
            run(Command::new("tar")
                .arg(format!("--format={format}"))
                .arg("-cf")
                .arg(scratch.path(name))
                .arg("-C")
                .arg(&src)
                .arg("."));
            archives.push(name);
        }
        run(Command::new("tar")
            .arg("-czf")
            .arg(scratch.path("gnu.tar.gz"))
            .arg("-C")
            .arg(&src)
            .arg("."));
        archives.push("gnu.tar.gz");
    }
    if available("bsdtar") {
        run(Command::new("bsdtar")
            .arg("-czf")
            .arg(scratch.path("bsd.tgz"))
            .arg("-C")
            .arg(&src)
            .arg("."));
        run(Command::new("bsdtar")
            .args(["--format", "zip", "-cf"])
            .arg(scratch.path("bsd.zip"))
            .arg("-C")
            .arg(&src)
            .arg("."));
        archives.extend(["bsd.tgz", "bsd.zip"]);
    }
    if available("zip") {
        run(Command::new("zip")
            .args(["-q", "-r", "-X"])
            .arg(scratch.path("info.zip"))
            .arg(".")
            .current_dir(&src));
        archives.push("info.zip");
    }
    if available("python3") {
        run(Command::new("python3")
            .arg("-c")
            .arg(
                "import os, sys, zipfile\n\
                 src, out = sys.argv[1], sys.argv[2]\n\
                 with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as archive:\n\
                 \x20   for root, dirs, files in os.walk(src):\n\
                 \x20       for name in dirs + files:\n\
                 \x20           path = os.path.join(root, name)\n\
                 \x20           archive.write(path, os.path.relpath(path, src))\n",
            )
            .arg(&src)
            .arg(scratch.path("python.zip")));
        archives.push("python.zip");
    }
    for name in archives {
        let destination = scratch.text(&format!("out-{name}"));
        unpack(&scratch.text(name), &destination, &UnpackOptions::default())
            .unwrap_or_else(|failure| panic!("{name}: {failure:?}"));
        assert_same_tree(Path::new(&destination), &files);
    }
}

#[test]
fn archives_written_by_tyrus_open_in_system_tools() {
    let scratch = Scratch::new();
    let files = sample(&scratch);
    for extension in ["zip", "tar", "tar.gz", "tgz"] {
        pack(
            &scratch.text("src"),
            &scratch.text(&format!("tyrus.{extension}")),
            &PackOptions::default(),
        )
        .unwrap();
    }
    let mut checked = 0;
    for (tool, archive) in [
        ("tar", "tyrus.tar"),
        ("tar", "tyrus.tar.gz"),
        ("tar", "tyrus.tgz"),
        ("bsdtar", "tyrus.tar"),
        ("bsdtar", "tyrus.tgz"),
        ("bsdtar", "tyrus.zip"),
        ("unzip", "tyrus.zip"),
    ] {
        if !available(tool) {
            continue;
        }
        let destination = scratch.path(&format!("{tool}-{archive}"));
        fs::create_dir_all(&destination).unwrap();
        if tool == "unzip" {
            run(Command::new("unzip")
                .arg("-q")
                .arg(scratch.path(archive))
                .arg("-d")
                .arg(&destination));
        } else {
            run(Command::new(tool)
                .arg("-xf")
                .arg(scratch.path(archive))
                .arg("-C")
                .arg(&destination));
        }
        assert_same_tree(&destination, &files);
        checked += 1;
    }
    if available("unzip") {
        run(Command::new("unzip")
            .arg("-tq")
            .arg(scratch.path("tyrus.zip")));
    }
    if available("python3") {
        run(Command::new("python3")
            .arg("-c")
            .arg("import sys, zipfile\nassert zipfile.ZipFile(sys.argv[1]).testzip() is None\n")
            .arg(scratch.path("tyrus.zip")));
    }
    if available("gzip") {
        run(Command::new("gzip")
            .arg("-t")
            .arg(scratch.path("tyrus.tgz")));
    }
    eprintln!("checked {checked} tool and archive pairs");
}
