//! Native end-to-end coverage for `tyrus:archive`: programs that pack, list
//! and unpack real archives, and that read every field of `ArchiveError`,
//! including the offending entry, from the `Err` every function resolves with.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct ArchiveProject {
    root: PathBuf,
    name: String,
}

impl ArchiveProject {
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
        let name = format!("archive-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Archive integration test" }},
  "security": {{ "envFile": ".env", "envSealed": false }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join(".env"), "").unwrap();
        fs::write(root.join("src/main.tyr"), source).unwrap();
        Self { root, name }
    }

    fn write(&self, relative: &str, contents: &[u8]) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn build_and_run(&self) -> Output {
        let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let mut output = Command::new(self.root.join("target/debug").join(&self.name))
            .current_dir(&self.root)
            .output()
            .unwrap();
        if cfg!(windows) {
            output.stdout = String::from_utf8(output.stdout)
                .unwrap()
                .replace("\r\n", "\n")
                .into_bytes();
        }
        output
    }
}

impl Drop for ArchiveProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A one-entry ustar archive whose entry name is written verbatim, the way a
/// hostile archive smuggles `../` past a well-behaved writer.
fn raw_tar(name: &str, contents: &[u8]) -> Vec<u8> {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    let size = format!("{:011o}\0", contents.len());
    header[124..136].copy_from_slice(size.as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    header[156] = b'0';
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    header[148..156].copy_from_slice(b"        ");
    let checksum: u32 = header.iter().map(|byte| u32::from(*byte)).sum();
    header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
    let mut archive = header.to_vec();
    archive.extend_from_slice(contents);
    archive.resize(archive.len().div_ceil(512) * 512, 0);
    archive.extend_from_slice(&[0u8; 1024]);
    archive
}

#[test]
fn a_program_packs_lists_and_unpacks_every_format() {
    let project = ArchiveProject::new(
        r#"import { ArchiveEntry, ArchiveEntryKind, ArchiveError, ArchiveSummary, PackOptions, UnpackOptions, list, pack, unpack } from "tyrus:archive";

const countFiles = (entries: ArchiveEntry[]): i32 => {
    let files = 0;
    for (const entry of entries) {
        if (entry.kind === ArchiveEntryKind.File) {
            files += 1;
        }
    }
    return files;
};

const roundTrip = async (destination: string, restored: string): Promise<Result<void, ArchiveError>> => {
    const packed = await pack("./data", destination, { exclude: ["**/*.log"], prefix: "bundle" })?;
    print(packed.entries, packed.bytes);
    const entries: ArchiveEntry[] = await list(destination)?;
    const done = await unpack(destination, restored, { stripComponents: 1 })?;
    print(countFiles(entries));
    print(done.entries, done.bytes);
    return Ok();
};

const report = (result: Result<void, ArchiveError>): void => {
    match (result) {
        Ok() => {}
        Err(error) => { printError(error.code); }
    }
};

const main = async (): Promise<void> => {
    report(await roundTrip("./out.zip", "./zip"));
    report(await roundTrip("./out.tar", "./tar"));
    report(await roundTrip("./out.tgz", "./tgz"));
};
"#,
    );
    project.write("data/readme.txt", b"hello");
    project.write("data/docs/guide.md", b"# guide\n");
    project.write("data/debug.log", b"ignored");
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "4 13\n2\n3 13\n".repeat(3)
    );
    for directory in ["zip", "tar", "tgz"] {
        assert_eq!(
            fs::read(project.root.join(directory).join("docs/guide.md")).unwrap(),
            b"# guide\n"
        );
        assert!(!project.root.join(directory).join("debug.log").exists());
    }
}

#[test]
fn every_err_carries_the_archive_error_and_its_entry() {
    let project = ArchiveProject::new(
        r#"import { ArchiveError, ArchiveSummary, unpack } from "tyrus:archive";

const extract = async (): Promise<Result<u64, ArchiveError>> => {
    const done = await unpack("./evil.tar", "./out")?;
    return Ok(done.entries);
};

const main = async (): Promise<void> => {
    match (await unpack("./evil.tar", "./out")) {
        Ok(summary) => { print(summary.entries); }
        Err(error) => { print(error.code, error.entry ?? "none"); }
    }
    match (await extract()) {
        Ok(entries) => { print(entries); }
        Err(error) => { print(error.message, error.entry ?? "none"); }
    }
    const missing = await unpack("./missing.zip", "./out");
    match (missing) {
        Ok(summary) => { print(summary.entries); }
        Err(error) => { print(error.code, error.entry ?? "none"); }
    }
};
"#,
    );
    project.write("evil.tar", &raw_tar("../escaped.txt", b"evil"));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_ARCHIVE_PATH_ESCAPE ../escaped.txt\n\
         an archive entry resolves outside the destination directory ../escaped.txt\n\
         ERR_ARCHIVE_NOT_FOUND none\n"
    );
    assert!(!project.root.join("escaped.txt").exists());
    assert!(!project.root.join("out").exists());
}

#[test]
fn onfail_binds_the_archive_error_and_then_the_summary() {
    let project = ArchiveProject::new(
        r#"import { ArchiveError, ArchiveSummary, unpack } from "tyrus:archive";

const extract = async (archive: string): Promise<Result<u64, ArchiveError>> => {
    const done = await unpack(archive, "./out") onfail {
        print("extract", done.code);
    };
    return Ok(done.entries);
};

const main = async (): Promise<void> => {
    match (await extract("./evil.tar")) {
        Ok(entries) => { print(entries); }
        Err(error) => { print(error.entry ?? "none"); }
    }
    const summary = await unpack("./missing.zip", "./out") onfail {
        print("main", summary.code);
    };
    print("unreachable", summary.entries);
};
"#,
    );
    project.write("evil.tar", &raw_tar("../escaped.txt", b"evil"));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "extract ERR_ARCHIVE_PATH_ESCAPE\n\
         ../escaped.txt\n\
         main ERR_ARCHIVE_NOT_FOUND\n"
    );
    assert!(!project.root.join("escaped.txt").exists());
}

#[test]
fn timeout_ms_reaches_the_runtime_for_every_operation() {
    let project = ArchiveProject::new(
        r#"import { ArchiveError, ArchiveSummary, PackOptions, UnpackOptions, list, pack, unpack } from "tyrus:archive";

const code = (result: Result<ArchiveSummary, ArchiveError>): string => {
    match (result) {
        Ok(summary) => { return `ok ${summary.entries}`; }
        Err(error) => { return error.code; }
    }
};

const both = async (): Promise<Result<u64, ArchiveError>> => {
    const packed = await pack("./data", "./ok.tgz", { reproducible: true, timeoutMs: 60000 })?;
    const done = await unpack("./ok.tgz", "./out", { maxRatio: 500, timeoutMs: 60000 })?;
    return Ok(packed.entries + done.entries);
};

const main = async (): Promise<void> => {
    print(code(await pack("./data", "./zero.zip", { level: 9, reproducible: true, timeoutMs: 0 })));
    match (await list("./data/readme.txt", { maxRatio: 500, timeoutMs: 0 })) {
        Ok(entries) => { print(entries.length); }
        Err(error) => { print(error.code); }
    }
    print(code(await unpack("./data/readme.txt", "./out", { maxRatio: 500, timeoutMs: 0 })));
    match (await both()) {
        Ok(total) => { print(total); }
        Err(error) => { printError(error.code); }
    }
};
"#,
    );
    project.write("data/readme.txt", b"hello");
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "ERR_ARCHIVE_INVALID_OPTION\nERR_ARCHIVE_INVALID_OPTION\nERR_ARCHIVE_INVALID_OPTION\n2\n"
    );
    assert!(!project.root.join("zero.zip").exists());
}

#[test]
fn archives_are_written_into_and_extracted_from_streams() {
    let project = ArchiveProject::new(
        r#"import { ArchiveError, ArchiveFormat, ArchiveSummary, PackOptions, UnpackOptions, packStream, unpackStream } from "tyrus:archive";
import { FsError, readFileStream, writeFileStream } from "tyrus:fs";
import { ReadableStream, StreamError, WritableStream } from "tyrus:stream";

const save = async (format: ArchiveFormat, destination: string): Promise<Result<void, FsError>> => {
    try {
        const archive: ReadableStream<u8[]> = packStream("./site", format, { reproducible: true });
        const sink: WritableStream<u8[]> = writeFileStream(destination)?;
        await archive.pipeTo(sink);
        print("packed", destination);
        return Ok();
    } catch (error: ArchiveError | StreamError) {
        print("failed", error.code);
        return Ok();
    }
};

const restore = async (source: string, destination: string): Promise<Result<void, FsError>> => {
    const input: ReadableStream<u8[]> = readFileStream(source)?;
    const result = await unpackStream(input, destination, { stripComponents: 0 });
    match (result) {
        Ok(summary) => { print("restored", summary.entries, summary.bytes); }
        Err(error) => { print(error.code, error.entry ?? "none"); }
    }
    return Ok();
};

const saveAbsent = async (): Promise<Result<void, FsError>> => {
    try {
        const absent: ReadableStream<u8[]> = packStream("./absent", ArchiveFormat.Tar);
        const sink: WritableStream<u8[]> = writeFileStream("./absent.tar")?;
        await absent.pipeTo(sink);
        return Ok();
    } catch (error: ArchiveError | StreamError) {
        print("absent", error.code);
        return Ok();
    }
};

const main = async (): Promise<void> => {
    const tgz = await save(ArchiveFormat.TarGz, "./site.tgz");
    match (tgz) { Ok() => {} Err(error) => { print("failed", error.code); } }
    const tar = await save(ArchiveFormat.Tar, "./site.tar");
    match (tar) { Ok() => {} Err(error) => { print("failed", error.code); } }
    const zip = await save(ArchiveFormat.Zip, "./site.zip");
    match (zip) { Ok() => {} Err(error) => { print("failed", error.code); } }
    const tgzRestore = await restore("./site.tgz", "./out-tgz");
    match (tgzRestore) { Ok() => {} Err(error) => { print(error.code); } }
    const tarRestore = await restore("./site.tar", "./out-tar");
    match (tarRestore) { Ok() => {} Err(error) => { print(error.code); } }
    const zipRestore = await restore("./site.zip", "./out-zip");
    match (zipRestore) { Ok() => {} Err(error) => { print(error.code); } }
    const evilRestore = await restore("./evil.tar", "./out-evil");
    match (evilRestore) { Ok() => {} Err(error) => { print(error.code); } }
    const missingSave = await save(ArchiveFormat.TarGz, "./missing/site.tgz");
    match (missingSave) { Ok() => {} Err(error) => { print("failed", error.code); } }
    const absentResult = await saveAbsent();
    match (absentResult) { Ok() => {} Err(error) => { print("absent", error.code); } }
};
"#,
    );
    project.write("site/index.html", b"<h1>tyrus</h1>");
    project.write("site/assets/app.js", b"console.log(1);");
    project.write("evil.tar", &raw_tar("../escaped.txt", b"evil"));
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "packed ./site.tgz\n\
         packed ./site.tar\n\
         packed ./site.zip\n\
         restored 3 29\n\
         restored 3 29\n\
         ERR_ARCHIVE_ZIP_NOT_STREAMABLE none\n\
         ERR_ARCHIVE_PATH_ESCAPE ../escaped.txt\n\
         failed ERR_FS_WRITE_FAILED\n\
         absent ERR_ARCHIVE_NOT_FOUND\n"
    );
    assert_eq!(
        fs::read(project.root.join("out-tgz/assets/app.js")).unwrap(),
        b"console.log(1);"
    );
    assert!(!project.root.join("escaped.txt").exists());
    assert!(!project.root.join("out-evil").exists());
    assert!(!project.root.join("out-zip").exists());
}
