//! Entry points for the `archive_list_unpack` fuzz target. Arbitrary bytes are
//! written as an archive and run through `list` and `unpack` with small
//! limits; beyond not panicking, every run must leave nothing outside the
//! destination, remove a destination it created when it fails, and account
//! for every byte it wrote.

use crate::archive::{UnpackOptions, list, unpack};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const MAX_ENTRIES: u64 = 256;
const MAX_BYTES: u64 = 8 * 1024 * 1024;

fn scratch() -> &'static PathBuf {
    static SCRATCH: OnceLock<PathBuf> = OnceLock::new();
    SCRATCH.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("tyrus-archive-fuzz-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fuzz scratch directory");
        root
    })
}

fn disk_bytes(root: &Path) -> u64 {
    let Ok(listing) = fs::read_dir(root) else {
        return 0;
    };
    listing
        .map(|entry| entry.expect("readable scratch entry").path())
        .map(|path| {
            let metadata = fs::symlink_metadata(&path).expect("scratch metadata");
            assert!(
                !metadata.is_symlink(),
                "extraction created a link: {path:?}"
            );
            if metadata.is_dir() {
                disk_bytes(&path)
            } else {
                metadata.len()
            }
        })
        .sum()
}

fn names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root)
        .expect("readable scratch")
        .map(|entry| {
            entry
                .expect("scratch entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

pub fn list_and_unpack(data: &[u8]) {
    let root = scratch();
    let archive = root.join("archive");
    let destination = root.join("out");
    fs::write(&archive, data).expect("write fuzz archive");
    let options = UnpackOptions {
        max_entries: Some(MAX_ENTRIES),
        max_bytes: Some(MAX_BYTES),
        timeout_ms: Some(10_000),
        ..UnpackOptions::default()
    };
    let archive_text = archive.to_string_lossy();
    if let Ok(entries) = list(&archive_text, &options) {
        assert!(entries.len() as u64 <= MAX_ENTRIES);
    }
    let destination_text = destination.to_string_lossy();
    match unpack(&archive_text, &destination_text, &options) {
        Ok(summary) => {
            assert!(summary.entries <= MAX_ENTRIES);
            assert!(summary.bytes <= MAX_BYTES);
            assert_eq!(disk_bytes(&destination), summary.bytes);
            assert_eq!(names(root), ["archive", "out"]);
        }
        Err(_) => assert_eq!(
            names(root),
            ["archive"],
            "a failed unpack left files behind"
        ),
    }
    let _ = fs::remove_dir_all(&destination);
}
