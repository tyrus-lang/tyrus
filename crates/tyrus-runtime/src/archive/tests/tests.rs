use super::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("tyrus-archive-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    fn text(&self, relative: &str) -> String {
        self.path(relative).to_string_lossy().into_owned()
    }

    fn write(&self, relative: &str, contents: &[u8]) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample_tree(scratch: &Scratch) {
    scratch.write("src/readme.txt", b"hello archive");
    scratch.write("src/nested/data.bin", &[7u8; 4096]);
    scratch.write("src/nested/app.js.map", b"{}");
    fs::create_dir_all(scratch.path("src/empty")).unwrap();
}

fn unpack_defaults() -> UnpackOptions {
    UnpackOptions::default()
}

fn code_of<T: std::fmt::Debug>(result: ArchiveResult<T>) -> &'static str {
    result.expect_err("operation should fail").code
}

fn round_trip(extension: &str) {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let archive = scratch.text(&format!("out/backup{extension}"));
    fs::create_dir_all(scratch.path("out")).unwrap();
    let packed = pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();
    assert_eq!(packed.entries, 5);
    assert_eq!(packed.bytes, 13 + 4096 + 2);

    let unpacked = unpack(&archive, &scratch.text("restored"), &unpack_defaults()).unwrap();
    assert_eq!(unpacked, packed);
    assert_eq!(
        fs::read(scratch.path("restored/readme.txt")).unwrap(),
        b"hello archive"
    );
    assert_eq!(
        fs::read(scratch.path("restored/nested/data.bin")).unwrap(),
        vec![7u8; 4096]
    );
    assert!(scratch.path("restored/empty").is_dir());

    let listed = list(&archive, &unpack_defaults()).unwrap();
    let mut names: Vec<_> = listed
        .iter()
        .map(|entry| entry.name.trim_end_matches('/'))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "empty",
            "nested",
            "nested/app.js.map",
            "nested/data.bin",
            "readme.txt"
        ]
    );
}

#[test]
fn zip_round_trips_files_and_empty_directories() {
    round_trip(".zip");
}

#[test]
fn tar_round_trips_files_and_empty_directories() {
    round_trip(".tar");
}

#[test]
fn tar_gz_round_trips_with_both_extensions() {
    round_trip(".tar.gz");
    round_trip(".tgz");
}

#[test]
fn list_reports_every_entry_with_its_kind_and_size() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let archive = scratch.text("a.zip");
    pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();
    let listed = list(&archive, &unpack_defaults()).unwrap();
    assert_eq!(listed.len(), 5);
    let data = listed
        .iter()
        .find(|entry| entry.name == "nested/data.bin")
        .unwrap();
    assert_eq!(data.kind, EntryKind::File);
    assert_eq!(data.size, 4096);
    assert!(data.compressed_size.unwrap() < 4096);
    let empty = listed.iter().find(|entry| entry.name == "empty/").unwrap();
    assert_eq!(empty.kind, EntryKind::Directory);
}

#[test]
fn pack_rejects_unknown_or_uppercase_extensions() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    for name in ["a.ZIP", "a.jar", "a.tar.GZ", "a"] {
        let result = pack(
            &scratch.text("src"),
            &scratch.text(name),
            &PackOptions::default(),
        );
        assert_eq!(code_of(result), ERR_UNSUPPORTED_EXTENSION, "{name}");
        assert!(!scratch.path(name).exists());
    }
}

#[test]
fn pack_validates_the_compression_level_per_format() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let options = |level| PackOptions {
        level: Some(level),
        ..PackOptions::default()
    };
    assert_eq!(
        code_of(pack(
            &scratch.text("src"),
            &scratch.text("a.zip"),
            &options(10)
        )),
        ERR_INVALID_COMPRESSION_LEVEL
    );
    assert_eq!(
        code_of(pack(
            &scratch.text("src"),
            &scratch.text("a.tgz"),
            &options(-1)
        )),
        ERR_INVALID_COMPRESSION_LEVEL
    );
    assert_eq!(
        code_of(pack(
            &scratch.text("src"),
            &scratch.text("a.tar"),
            &options(1)
        )),
        ERR_INVALID_COMPRESSION_LEVEL
    );
    pack(
        &scratch.text("src"),
        &scratch.text("stored.zip"),
        &options(0),
    )
    .unwrap();
    pack(
        &scratch.text("src"),
        &scratch.text("best.tar.gz"),
        &options(9),
    )
    .unwrap();
}

#[test]
fn pack_refuses_an_existing_destination_unless_overwrite_is_set() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    scratch.write("a.zip", b"previous");
    let result = pack(
        &scratch.text("src"),
        &scratch.text("a.zip"),
        &PackOptions::default(),
    );
    assert_eq!(code_of(result), ERR_DESTINATION_EXISTS);
    assert_eq!(fs::read(scratch.path("a.zip")).unwrap(), b"previous");

    let overwrite = PackOptions {
        overwrite: true,
        ..PackOptions::default()
    };
    pack(&scratch.text("src"), &scratch.text("a.zip"), &overwrite).unwrap();
    assert!(list(&scratch.text("a.zip"), &unpack_defaults()).is_ok());

    fs::create_dir_all(scratch.path("dir.zip")).unwrap();
    let result = pack(&scratch.text("src"), &scratch.text("dir.zip"), &overwrite);
    assert_eq!(code_of(result), ERR_DESTINATION_EXISTS);
}

#[test]
fn pack_skips_its_own_destination_inside_the_source_tree() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let archive = scratch.text("src/self.zip");
    pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();
    let listed = list(&archive, &unpack_defaults()).unwrap();
    assert!(listed.iter().all(|entry| !entry.name.contains("self.zip")));
    assert!(listed.iter().all(|entry| !entry.name.contains(".tmp")));
}

#[test]
fn pack_applies_exclude_globs_and_prefix() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let archive = scratch.text("release.tar.gz");
    let options = PackOptions {
        exclude: vec!["**/*.map".to_owned()],
        prefix: Some("release-1.0.0".to_owned()),
        ..PackOptions::default()
    };
    pack(&scratch.text("src"), &archive, &options).unwrap();
    let listed = list(&archive, &unpack_defaults()).unwrap();
    assert!(
        listed
            .iter()
            .all(|entry| entry.name.starts_with("release-1.0.0"))
    );
    assert!(listed.iter().all(|entry| !entry.name.ends_with(".map")));

    let strip = UnpackOptions {
        strip_components: 1,
        ..unpack_defaults()
    };
    unpack(&archive, &scratch.text("out"), &strip).unwrap();
    assert!(scratch.path("out/readme.txt").is_file());
    assert!(!scratch.path("out/release-1.0.0").exists());
}

#[test]
fn pack_rejects_a_prefix_that_escapes() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let options = PackOptions {
        prefix: Some("../up".to_owned()),
        ..PackOptions::default()
    };
    let result = pack(&scratch.text("src"), &scratch.text("a.zip"), &options);
    assert_eq!(code_of(result), ERR_INVALID_OPTION);
}

#[test]
fn pack_accepts_a_single_file_source() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    pack(
        &scratch.text("src/readme.txt"),
        &scratch.text("one.zip"),
        &PackOptions::default(),
    )
    .unwrap();
    let listed = list(&scratch.text("one.zip"), &unpack_defaults()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "readme.txt");
}

#[test]
fn pack_reports_a_missing_source() {
    let scratch = Scratch::new();
    let result = pack(
        &scratch.text("missing"),
        &scratch.text("a.zip"),
        &PackOptions::default(),
    );
    assert_eq!(code_of(result), ERR_NOT_FOUND);
    assert!(!scratch.path("a.zip").exists());
}

#[test]
fn reproducible_archives_are_byte_identical() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let options = PackOptions {
        reproducible: true,
        ..PackOptions::default()
    };
    for extension in [".zip", ".tar", ".tar.gz"] {
        let first = scratch.text(&format!("first{extension}"));
        let second = scratch.text(&format!("second{extension}"));
        pack(&scratch.text("src"), &first, &options).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        filetime_touch(&scratch.path("src/readme.txt"));
        pack(&scratch.text("src"), &second, &options).unwrap();
        assert_eq!(
            fs::read(first).unwrap(),
            fs::read(second).unwrap(),
            "{extension}"
        );
    }
}

fn filetime_touch(path: &Path) {
    let contents = fs::read(path).unwrap();
    fs::write(path, contents).unwrap();
}

#[cfg(unix)]
#[test]
fn pack_refuses_symbolic_links_in_the_source() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    std::os::unix::fs::symlink("/etc/passwd", scratch.path("src/link")).unwrap();
    let result = pack(
        &scratch.text("src"),
        &scratch.text("a.zip"),
        &PackOptions::default(),
    );
    let failure = result.unwrap_err();
    assert_eq!(failure.code, ERR_UNSUPPORTED_ENTRY);
    assert_eq!(failure.entry.as_deref(), Some("link"));
}

#[cfg(unix)]
#[test]
fn unpack_keeps_only_the_owner_execute_bit() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new();
    scratch.write("src/tool", b"#!/bin/sh\n");
    fs::set_permissions(scratch.path("src/tool"), fs::Permissions::from_mode(0o4775)).unwrap();
    scratch.write("src/plain", b"x");
    for extension in [".zip", ".tar.gz"] {
        let archive = scratch.text(&format!("a{extension}"));
        pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();
        let out = format!("out{extension}");
        unpack(&archive, &scratch.text(&out), &unpack_defaults()).unwrap();
        let tool = fs::metadata(scratch.path(&format!("{out}/tool"))).unwrap();
        let plain = fs::metadata(scratch.path(&format!("{out}/plain"))).unwrap();
        assert_eq!(tool.permissions().mode() & 0o7100, 0o100, "{extension}");
        assert_eq!(plain.permissions().mode() & 0o7111, 0, "{extension}");
    }
}

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, contents) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn tar_with(build: impl FnOnce(&mut tar::Builder<Vec<u8>>)) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    build(&mut builder);
    builder.into_inner().unwrap()
}

fn tar_file(builder: &mut tar::Builder<Vec<u8>>, name: &str, contents: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    builder.append_data(&mut header, name, contents).unwrap();
}

/// Writes a raw GNU header whose name field holds `name` verbatim, bypassing
/// the path checks `tar::Builder::append_data` applies, the way a hostile
/// archive would.
fn tar_raw_file(builder: &mut tar::Builder<Vec<u8>>, name: &str, contents: &[u8]) {
    let mut header = tar::Header::new_gnu();
    header.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
    header.set_size(contents.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    header.set_cksum();
    builder.append(&header, contents).unwrap();
}

fn unpack_bytes(
    scratch: &Scratch,
    name: &str,
    bytes: &[u8],
    options: &UnpackOptions,
) -> ArchiveResult<Summary> {
    scratch.write(name, bytes);
    unpack(&scratch.text(name), &scratch.text("out"), options)
}

#[test]
fn unpack_detects_the_format_by_content_not_by_name() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n")]);
    unpack_bytes(&scratch, "library.jar", &bytes, &unpack_defaults()).unwrap();
    assert!(scratch.path("out/META-INF/MANIFEST.MF").is_file());

    let tar = tar_with(|builder| tar_file(builder, "a.txt", b"a"));
    scratch.write("misnamed.zip", &tar);
    assert!(
        unpack(
            &scratch.text("misnamed.zip"),
            &scratch.text("out2"),
            &unpack_defaults()
        )
        .is_ok()
    );
}

#[test]
fn unpack_rejects_content_it_does_not_recognize() {
    let scratch = Scratch::new();
    let result = unpack_bytes(&scratch, "notes.zip", b"just some text", &unpack_defaults());
    assert_eq!(code_of(result), ERR_UNSUPPORTED_FORMAT);
    assert!(!scratch.path("out").exists());
}

#[test]
fn unpack_blocks_zip_slip_in_zip_and_tar() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("ok.txt", b"ok"), ("../../escaped.txt", b"evil")]);
    let failure = unpack_bytes(&scratch, "slip.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_PATH_ESCAPE);
    assert_eq!(failure.entry.as_deref(), Some("../../escaped.txt"));
    assert!(!scratch.0.parent().unwrap().join("escaped.txt").exists());
    assert!(
        !scratch.path("out").exists(),
        "rollback removes the created root"
    );

    let tar = tar_with(|builder| tar_raw_file(builder, "../escaped.txt", b"evil"));
    let failure = unpack_bytes(&scratch, "slip.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_PATH_ESCAPE);
    let tar = tar_with(|builder| tar_raw_file(builder, "/etc/escaped", b"evil"));
    let failure = unpack_bytes(&scratch, "abs.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_PATH_ESCAPE);
}

#[test]
fn unpack_refuses_links_inside_the_archive() {
    let scratch = Scratch::new();
    let tar = tar_with(|builder| {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        builder
            .append_link(&mut header, "link", "/etc/passwd")
            .unwrap();
    });
    let failure = unpack_bytes(&scratch, "link.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_UNSAFE_LINK);
    assert_eq!(failure.entry.as_deref(), Some("link"));

    let tar = tar_with(|builder| {
        tar_file(builder, "target", b"x");
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Link);
        header.set_size(0);
        builder.append_link(&mut header, "hard", "target").unwrap();
    });
    let failure = unpack_bytes(&scratch, "hard.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_UNSAFE_LINK);

    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .add_symlink(
            "zlink",
            "/etc/passwd",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
    let bytes = writer.finish().unwrap().into_inner();
    let failure = unpack_bytes(&scratch, "link.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_UNSAFE_LINK);
}

#[cfg(unix)]
#[test]
fn unpack_does_not_follow_a_symlink_already_in_the_destination() {
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.path("out")).unwrap();
    fs::create_dir_all(scratch.path("outside")).unwrap();
    std::os::unix::fs::symlink(scratch.path("outside"), scratch.path("out/docs")).unwrap();
    let bytes = zip_with(&[("docs/planted.txt", b"evil")]);
    let failure = unpack_bytes(&scratch, "a.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_PATH_ESCAPE);
    assert!(!scratch.path("outside/planted.txt").exists());
    assert!(
        scratch.path("out/docs").is_symlink(),
        "pre-existing content is left alone"
    );
}

#[test]
fn unpack_rejects_case_and_normalization_collisions() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("Readme.txt", b"a"), ("README.TXT", b"b")]);
    let failure = unpack_bytes(&scratch, "case.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_DUPLICATE_ENTRY);
    assert_eq!(failure.entry.as_deref(), Some("README.TXT"));

    let tar = tar_with(|builder| {
        tar_file(builder, "caf\u{e9}.txt", b"a");
        tar_file(builder, "cafe\u{301}.txt", b"b");
    });
    let failure = unpack_bytes(&scratch, "nfc.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_DUPLICATE_ENTRY);

    let tar = tar_with(|builder| {
        tar_file(builder, "same.txt", b"a");
        tar_file(builder, "same.txt", b"b");
    });
    let failure = unpack_bytes(&scratch, "dup.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_DUPLICATE_ENTRY);

    let tar = tar_with(|builder| {
        tar_file(builder, "a", b"file");
        tar_file(builder, "a/b", b"child");
    });
    let failure = unpack_bytes(&scratch, "shape.tar", &tar, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_DUPLICATE_ENTRY);
}

#[test]
fn unpack_rejects_names_that_are_not_portable() {
    let scratch = Scratch::new();
    let tar = tar_with(|builder| tar_raw_file(builder, "dir/CON", b"x"));
    let failure = unpack_bytes(
        &scratch,
        "reserved-device-entry.tar",
        &tar,
        &unpack_defaults(),
    )
    .unwrap_err();
    assert_eq!(failure.code, ERR_INVALID_ENTRY_NAME);
    assert_eq!(failure.entry.as_deref(), Some("dir/CON"));
}

#[test]
fn unpack_enforces_the_entry_limit() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);
    let options = UnpackOptions {
        max_entries: Some(2),
        ..unpack_defaults()
    };
    let result = unpack_bytes(&scratch, "many.zip", &bytes, &options);
    assert_eq!(code_of(result), ERR_ENTRY_LIMIT);
    assert_eq!(
        code_of(list(&scratch.text("many.zip"), &options)),
        ERR_ENTRY_LIMIT
    );

    let tar = tar_with(|builder| {
        for name in ["a", "b", "c"] {
            tar_file(builder, name, b"x");
        }
    });
    let result = unpack_bytes(&scratch, "many.tar", &tar, &options);
    assert_eq!(code_of(result), ERR_ENTRY_LIMIT);
    assert!(!scratch.path("out").exists());
}

#[test]
fn unpack_enforces_the_size_limit_on_written_bytes() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("a.bin", &[1u8; 600]), ("b.bin", &[2u8; 600])]);
    let options = UnpackOptions {
        max_bytes: Some(1000),
        ..unpack_defaults()
    };
    let failure = unpack_bytes(&scratch, "big.zip", &bytes, &options).unwrap_err();
    assert_eq!(failure.code, ERR_SIZE_LIMIT);
    assert_eq!(failure.entry.as_deref(), Some("b.bin"));
    assert!(!scratch.path("out").exists());
}

#[test]
fn unpack_enforces_the_compression_ratio() {
    let scratch = Scratch::new();
    let zeros = vec![0u8; 4 * 1024 * 1024];
    let bytes = zip_with(&[("bomb.bin", &zeros)]);
    let failure = unpack_bytes(&scratch, "bomb.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_RATIO_LIMIT);
    let generous = UnpackOptions {
        max_ratio: Some(100_000),
        ..unpack_defaults()
    };
    unpack(&scratch.text("bomb.zip"), &scratch.text("out"), &generous).unwrap();
}

#[test]
fn unpack_rejects_zero_limits() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("a", b"1")]);
    for options in [
        UnpackOptions {
            max_entries: Some(0),
            ..unpack_defaults()
        },
        UnpackOptions {
            max_bytes: Some(0),
            ..unpack_defaults()
        },
        UnpackOptions {
            max_ratio: Some(0),
            ..unpack_defaults()
        },
    ] {
        let result = unpack_bytes(&scratch, "a.zip", &bytes, &options);
        assert_eq!(code_of(result), ERR_INVALID_OPTION);
    }
}

/// Offsets of every central-directory record in a ZIP the tests built.
fn central_records(bytes: &[u8]) -> Vec<usize> {
    bytes
        .windows(4)
        .enumerate()
        .filter(|(_, window)| *window == b"PK\x01\x02")
        .map(|(index, _)| index)
        .collect()
}

#[test]
fn unpack_detects_a_checksum_mismatch() {
    let scratch = Scratch::new();
    let mut bytes = zip_with(&[("data.txt", b"the original contents")]);
    let local_crc = 14;
    bytes[local_crc] ^= 0xFF;
    let central = central_records(&bytes)[0];
    bytes[central + 16] ^= 0xFF;
    let failure = unpack_bytes(&scratch, "crc.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_CHECKSUM_MISMATCH);
    assert_eq!(failure.entry.as_deref(), Some("data.txt"));
    assert!(!scratch.path("out").exists());
}

#[test]
fn unpack_refuses_encrypted_entries() {
    let scratch = Scratch::new();
    let mut bytes = zip_with(&[("secret.txt", b"not really encrypted")]);
    bytes[6] |= 1;
    let central = central_records(&bytes)[0];
    bytes[central + 8] |= 1;
    let failure = unpack_bytes(&scratch, "enc.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_ENCRYPTED);
    assert_eq!(failure.entry.as_deref(), Some("secret.txt"));
}

#[test]
fn unpack_rejects_overlapping_zip_entries() {
    let scratch = Scratch::new();
    let bytes = zip_with(&[("first.txt", b"aaaa"), ("second.txt", b"bbbb")]);
    let records = central_records(&bytes);
    let mut patched = bytes.clone();
    let first_offset = bytes[records[0] + 42..records[0] + 46].to_vec();
    patched[records[1] + 42..records[1] + 46].copy_from_slice(&first_offset);
    let result = unpack_bytes(&scratch, "overlap.zip", &patched, &unpack_defaults());
    assert_eq!(code_of(result), ERR_INVALID_ARCHIVE);
}

#[test]
fn unpack_rejects_a_truncated_archive_and_rolls_back() {
    let scratch = Scratch::new();
    let tar = tar_with(|builder| {
        tar_file(builder, "first.txt", b"complete");
        tar_file(builder, "second.bin", &[3u8; 8192]);
    });
    let truncated = &tar[..512 + 512 + 512 + 1000];
    let result = unpack_bytes(&scratch, "cut.tar", truncated, &unpack_defaults());
    assert_eq!(code_of(result), ERR_INVALID_ARCHIVE);
    assert!(!scratch.path("out").exists());
}

#[test]
fn rollback_leaves_pre_existing_destination_content_alone() {
    let scratch = Scratch::new();
    scratch.write("out/keep.txt", b"mine");
    let bytes = zip_with(&[("new/a.txt", b"a"), ("../evil", b"x")]);
    let result = unpack_bytes(&scratch, "a.zip", &bytes, &unpack_defaults());
    assert_eq!(code_of(result), ERR_PATH_ESCAPE);
    assert_eq!(fs::read(scratch.path("out/keep.txt")).unwrap(), b"mine");
    assert!(!scratch.path("out/new").exists());
}

#[test]
fn unpack_respects_overwrite_for_existing_files() {
    let scratch = Scratch::new();
    scratch.write("out/a.txt", b"old");
    let bytes = zip_with(&[("a.txt", b"new")]);
    let failure = unpack_bytes(&scratch, "a.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_DESTINATION_EXISTS);
    assert_eq!(failure.entry.as_deref(), Some("a.txt"));
    assert_eq!(fs::read(scratch.path("out/a.txt")).unwrap(), b"old");

    let overwrite = UnpackOptions {
        overwrite: true,
        ..unpack_defaults()
    };
    unpack(&scratch.text("a.zip"), &scratch.text("out"), &overwrite).unwrap();
    assert_eq!(fs::read(scratch.path("out/a.txt")).unwrap(), b"new");
}

#[test]
fn unpack_refuses_a_destination_that_is_a_file() {
    let scratch = Scratch::new();
    scratch.write("out", b"a file");
    let bytes = zip_with(&[("a.txt", b"a")]);
    let result = unpack_bytes(&scratch, "a.zip", &bytes, &unpack_defaults());
    assert_eq!(code_of(result), ERR_DESTINATION_EXISTS);
}

#[test]
fn errors_truncate_long_entry_names() {
    let failure = ArchiveFailure::at(ERR_PATH_ESCAPE, &"é".repeat(5000));
    let entry = failure.entry.unwrap();
    assert!(entry.len() <= MAX_ERROR_ENTRY_BYTES);
    assert!(entry.chars().all(|character| character == 'é'));
}

#[test]
fn every_code_has_its_own_message() {
    for code in [
        ERR_NOT_FOUND,
        ERR_IO,
        ERR_PERMISSION_DENIED,
        ERR_UNSUPPORTED_FORMAT,
        ERR_UNSUPPORTED_EXTENSION,
        ERR_INVALID_ARCHIVE,
        ERR_CHECKSUM_MISMATCH,
        ERR_ENCRYPTED,
        ERR_INVALID_OPTION,
        ERR_INVALID_COMPRESSION_LEVEL,
        ERR_DESTINATION_EXISTS,
        ERR_PATH_ESCAPE,
        ERR_INVALID_ENTRY_NAME,
        ERR_DUPLICATE_ENTRY,
        ERR_UNSAFE_LINK,
        ERR_UNSUPPORTED_ENTRY,
        ERR_ENTRY_LIMIT,
        ERR_SIZE_LIMIT,
        ERR_RATIO_LIMIT,
        ERR_ALLOCATION_FAILED,
        ERR_NO_SPACE,
        ERR_TIMEOUT,
        ERR_ZIP_NOT_STREAMABLE,
    ] {
        assert_ne!(message_for(code), message_for("unknown"), "{code}");
    }
}

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

#[test]
fn tar_gz_reads_every_gzip_member_and_stops_at_the_end_of_the_archive() {
    let scratch = Scratch::new();
    let archive = tar_with(|builder| tar_file(builder, "notes.txt", b"members"));
    let (first, second) = archive.split_at(700);
    let members = [gzip(first), gzip(second)].concat();
    let summary = unpack_bytes(&scratch, "members.tgz", &members, &unpack_defaults()).unwrap();
    assert_eq!(summary.entries, 1);
    assert_eq!(fs::read(scratch.path("out/notes.txt")).unwrap(), b"members");
    fs::remove_dir_all(scratch.path("out")).unwrap();

    // Reading stops at the end-of-archive marker, so bytes appended after the
    // gzip stream are never read, as with GNU tar.
    let mut trailing = gzip(&archive);
    trailing.extend_from_slice(b"appended after the stream");
    let summary = unpack_bytes(&scratch, "trailing.tgz", &trailing, &unpack_defaults()).unwrap();
    assert_eq!(summary.entries, 1);
}

#[test]
fn tar_gz_enforces_the_compression_ratio_over_the_whole_stream() {
    let scratch = Scratch::new();
    let zeros = vec![0u8; 4 * 1024 * 1024];
    let single = gzip(&tar_with(|builder| tar_file(builder, "bomb.bin", &zeros)));
    let failure = unpack_bytes(&scratch, "single.tgz", &single, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_RATIO_LIMIT);
    assert!(!scratch.path("out").exists());

    // Every member stays under the 1 MiB floor a per-entry check would need;
    // only the stream as a whole gives the bomb away.
    let chunk = vec![0u8; 64 * 1024];
    let spread = gzip(&tar_with(|builder| {
        for index in 0..64 {
            tar_file(builder, &format!("part-{index}.bin"), &chunk);
        }
    }));
    let failure = unpack_bytes(&scratch, "spread.tgz", &spread, &unpack_defaults()).unwrap_err();
    assert_eq!(failure.code, ERR_RATIO_LIMIT);
    assert!(!scratch.path("out").exists());

    let generous = UnpackOptions {
        max_ratio: Some(100_000),
        ..unpack_defaults()
    };
    let summary = unpack(&scratch.text("spread.tgz"), &scratch.text("out"), &generous).unwrap();
    assert_eq!(summary.entries, 64);
}

#[test]
fn an_uncompressed_tar_is_never_refused_for_its_ratio() {
    let scratch = Scratch::new();
    let zeros = vec![0u8; 4 * 1024 * 1024];
    let bytes = tar_with(|builder| tar_file(builder, "zeros.bin", &zeros));
    let summary = unpack_bytes(&scratch, "zeros.tar", &bytes, &unpack_defaults()).unwrap();
    assert_eq!(summary.bytes, zeros.len() as u64);
}

#[test]
fn every_operation_stops_at_its_deadline_and_rolls_back() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    fs::create_dir_all(scratch.path("out")).unwrap();
    for extension in [".zip", ".tar", ".tgz"] {
        let archive = scratch.text(&format!("out/a{extension}"));
        pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();

        let expired = Deadline::expired();
        let listed = super::unpack::list_until(&archive, &unpack_defaults(), expired);
        assert_eq!(code_of(listed), ERR_TIMEOUT, "{extension}");
        let restored = scratch.text(&format!("restored{extension}"));
        let unpacked =
            super::unpack::unpack_until(&archive, &restored, &unpack_defaults(), expired);
        assert_eq!(code_of(unpacked), ERR_TIMEOUT, "{extension}");
        assert!(!Path::new(&restored).exists(), "{extension}");

        let late = scratch.text(&format!("out/late{extension}"));
        let packed = super::pack::pack_until(
            &scratch.text("src"),
            &late,
            &PackOptions::default(),
            expired,
        );
        assert_eq!(code_of(packed), ERR_TIMEOUT, "{extension}");
        assert!(!Path::new(&late).exists(), "{extension}");
    }
    let leftovers: Vec<_> = fs::read_dir(scratch.path("out"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn the_timeout_can_be_raised_but_not_switched_off() {
    let scratch = Scratch::new();
    sample_tree(&scratch);
    let archive = scratch.text("a.zip");
    let zero = PackOptions {
        timeout_ms: Some(0),
        ..PackOptions::default()
    };
    assert_eq!(
        code_of(pack(&scratch.text("src"), &archive, &zero)),
        ERR_INVALID_OPTION
    );
    let huge = PackOptions {
        timeout_ms: Some(u64::MAX),
        ..PackOptions::default()
    };
    pack(&scratch.text("src"), &archive, &huge).unwrap();

    let zero = UnpackOptions {
        timeout_ms: Some(0),
        ..unpack_defaults()
    };
    assert_eq!(code_of(list(&archive, &zero)), ERR_INVALID_OPTION);
    assert_eq!(
        code_of(unpack(&archive, &scratch.text("out"), &zero)),
        ERR_INVALID_OPTION
    );
    let huge = UnpackOptions {
        timeout_ms: Some(u64::MAX),
        ..unpack_defaults()
    };
    assert_eq!(list(&archive, &huge).unwrap().len(), 5);
    assert_eq!(
        unpack(&archive, &scratch.text("out"), &huge)
            .unwrap()
            .entries,
        5
    );
}

#[test]
fn unpack_refuses_a_zip_that_cannot_fit_before_writing_anything() {
    let scratch = Scratch::new();
    let names: Vec<String> = (0..10_000).map(|index| format!("f{index}")).collect();
    let entries: Vec<(&str, &[u8])> = names
        .iter()
        .map(|name| (name.as_str(), &b"x"[..]))
        .collect();
    let mut bytes = zip_with(&entries);
    // Each central record now claims almost 4 GiB, about 43 TB in total,
    // more than any test machine has free.
    for record in central_records(&bytes) {
        bytes[record + 24..record + 28].copy_from_slice(&0xFFFF_FFFEu32.to_le_bytes());
    }
    let options = UnpackOptions {
        max_bytes: Some(u64::MAX),
        ..unpack_defaults()
    };
    let failure = unpack_bytes(&scratch, "huge.zip", &bytes, &options).unwrap_err();
    assert_eq!(failure.code, ERR_NO_SPACE);
    assert!(!scratch.path("out").exists());

    // Under the default `maxBytes` the declared total is capped at 4 GiB,
    // which fits, and the lie is caught while the first entry is written.
    let failure = unpack_bytes(&scratch, "huge.zip", &bytes, &unpack_defaults()).unwrap_err();
    assert_ne!(failure.code, ERR_NO_SPACE);
    assert!(!scratch.path("out").exists());
}

#[test]
fn a_full_disk_and_a_timeout_have_their_own_codes() {
    for (kind, code) in [
        (std::io::ErrorKind::StorageFull, ERR_NO_SPACE),
        (std::io::ErrorKind::QuotaExceeded, ERR_NO_SPACE),
        (std::io::ErrorKind::TimedOut, ERR_TIMEOUT),
    ] {
        assert_eq!(io_failure(&kind.into()).code, code);
        assert_eq!(io_failure_at(&kind.into(), "a").entry.as_deref(), Some("a"));
    }
}

/// Every path below `root`, relative and sorted, so a test can assert that a
/// failed call left the tree exactly as it found it.
fn tree(root: &Path) -> Vec<String> {
    fn walk(root: &Path, directory: &Path, paths: &mut Vec<String>) {
        let Ok(listing) = fs::read_dir(directory) else {
            return;
        };
        for entry in listing {
            let path = entry.unwrap().path();
            paths.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            if path.is_dir() && !path.is_symlink() {
                walk(root, &path, paths);
            }
        }
    }
    let mut paths = Vec::new();
    walk(root, root, &mut paths);
    paths.sort();
    paths
}

fn larger_tree(scratch: &Scratch) {
    for index in 0..8 {
        scratch.write(&format!("src/part-{index}.bin"), &[index as u8; 16 * 1024]);
    }
}

#[test]
fn a_disk_that_fills_up_during_unpack_rolls_back_every_format() {
    let scratch = Scratch::new();
    larger_tree(&scratch);
    fs::create_dir_all(scratch.path("archives")).unwrap();
    for extension in [".zip", ".tar", ".tgz"] {
        let archive = scratch.text(&format!("archives/a{extension}"));
        pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();

        let fresh = scratch.text(&format!("fresh{extension}"));
        let failure = with_write_budget(40 * 1024, || unpack(&archive, &fresh, &unpack_defaults()))
            .unwrap_err();
        assert_eq!(failure.code, ERR_NO_SPACE, "{extension}");
        assert!(failure.entry.is_some(), "{extension}");
        assert!(!Path::new(&fresh).exists(), "{extension}");

        let existing = scratch.path(&format!("existing{extension}"));
        fs::create_dir_all(existing.join("kept")).unwrap();
        fs::write(existing.join("kept/note.txt"), b"mine").unwrap();
        let before = tree(&existing);
        let failure = with_write_budget(40 * 1024, || {
            unpack(&archive, &existing.to_string_lossy(), &unpack_defaults())
        })
        .unwrap_err();
        assert_eq!(failure.code, ERR_NO_SPACE, "{extension}");
        assert_eq!(tree(&existing), before, "{extension}");
        assert_eq!(fs::read(existing.join("kept/note.txt")).unwrap(), b"mine");
    }
}

#[cfg(any(unix, windows))]
#[test]
fn zip_space_preflight_reads_available_bytes_from_the_destination_volume() {
    let scratch = Scratch::new();
    let root = cap_std::fs::Dir::open_ambient_dir(&scratch.0, cap_std::ambient_authority())
        .expect("the temporary destination directory must open");
    let available = super::unpack::available_space(&root, &scratch.0)
        .expect("Windows must report free space for the destination volume");
    assert!(available > 0);
}

#[test]
fn a_disk_that_fills_up_during_pack_leaves_no_archive_or_temporary_file() {
    let scratch = Scratch::new();
    larger_tree(&scratch);
    fs::create_dir_all(scratch.path("out")).unwrap();
    for extension in [".zip", ".tar", ".tgz"] {
        let destination = scratch.text(&format!("out/a{extension}"));
        // Level 0 keeps the gzip and deflate output as large as the input, so
        // the budget runs out mid-archive for every format.
        let options = PackOptions {
            level: (extension != ".tar").then_some(0),
            ..PackOptions::default()
        };
        let failure = with_write_budget(40 * 1024, || {
            pack(&scratch.text("src"), &destination, &options)
        })
        .unwrap_err();
        assert_eq!(failure.code, ERR_NO_SPACE, "{extension}");
        assert!(tree(&scratch.path("out")).is_empty(), "{extension}");
    }
}

#[test]
fn the_write_budget_is_scoped_to_the_call() {
    let scratch = Scratch::new();
    larger_tree(&scratch);
    let archive = scratch.text("a.tar");
    assert_eq!(
        code_of(with_write_budget(0, || pack(
            &scratch.text("src"),
            &archive,
            &PackOptions::default()
        ))),
        ERR_NO_SPACE
    );
    pack(&scratch.text("src"), &archive, &PackOptions::default()).unwrap();
}
