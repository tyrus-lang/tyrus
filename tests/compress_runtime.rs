//! Native end-to-end coverage for `tyrus:compress`: programs that compress and
//! decompress in every format, read every `CompressError` from the `Err` each
//! function resolves with, and reach the decompression limits.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct CompressProject {
    root: PathBuf,
    name: String,
}

impl CompressProject {
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
        let name = format!("compress-runtime-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{ "name": "{name}", "version": "0.1.0", "authors": [], "description": "Compress integration test" }},
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
        Command::new(self.root.join("target/debug").join(&self.name))
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
}

impl Drop for CompressProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn stdout_of(source: &str) -> String {
    let output = CompressProject::new(source).build_and_run();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn every_format_round_trips_text_and_bytes() {
    let stdout = stdout_of(
        r#"import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, decompress } from "tyrus:compress";
import { utf8 } from "tyrus:encoding";

const roundTrip = async (data: u8[] | string, format: CompressFormat, level: i32): Promise<Result<string, CompressError>> => {
    const packed = await compress(data, format, { level: level })?;
    const options: DecompressOptions = { format: format };
    const restored = await decompress(packed, options)?;
    return Ok(utf8.decodeLossy(restored));
};

const main = async (): Promise<void> => {
    const formats: CompressFormat[] = [CompressFormat.Gzip, CompressFormat.Deflate, CompressFormat.DeflateRaw, CompressFormat.Brotli];
    for (const format of formats) {
        match (await roundTrip("Hello, Tyrus!", format, 1)) {
            Ok(text) => { print(text); }
            Err(error) => { print(error.code); }
        }
        const bytes: u8[] = [116, 121, 114, 117, 115];
        match (await roundTrip(bytes, format, 9)) {
            Ok(text) => { print(text); }
            Err(error) => { print(error.code); }
        }
    }
    const zlib = await compress("detected", CompressFormat.Deflate) onfail {
        print(zlib.code);
    };
    const detected = await decompress(zlib) onfail {
        print(detected.code);
    };
    print(utf8.decodeLossy(detected));
    const brotli = await compress("undetectable", CompressFormat.Brotli) onfail {
        print(brotli.code);
    };
    match (await decompress(brotli)) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
};
"#,
    );
    assert_eq!(
        stdout,
        "Hello, Tyrus!\ntyrus\n".repeat(4) + "detected\nERR_COMPRESS_UNKNOWN_FORMAT\n"
    );
}

#[test]
fn onfail_and_match_read_every_error_code() {
    let stdout = stdout_of(
        r#"import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, decompress } from "tyrus:compress";

const levelOf = (value: i32): i32 => value;

const flipped = (bytes: u8[], index: usize): u8[] => {
    let copy: u8[] = bytes.slice(0);
    copy[index] = copy[index] ^ 255;
    return copy;
};

const main = async (): Promise<void> => {
    match (await decompress([1, 2, 3])) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code, error.message); }
    }
    const raw: DecompressOptions = { format: CompressFormat.DeflateRaw };
    match (await decompress([255, 255, 255], raw)) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
    match (await compress("x", CompressFormat.Brotli, { level: levelOf(12) })) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
    const packed = await compress("checksummed", CompressFormat.Gzip) onfail {
        print(packed.code);
    };
    match (await decompress(flipped(packed, packed.length - 8))) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
    const zero: DecompressOptions = { maxRatio: 0 };
    const refused = await decompress(packed, zero) onfail {
        print(refused.code);
        return;
    };
    print("unreachable", refused.length);
};
"#,
    );
    assert_eq!(
        stdout,
        "ERR_COMPRESS_UNKNOWN_FORMAT the format cannot be detected from the data; pass `format` in the options\n\
         ERR_COMPRESS_INVALID_DATA\n\
         ERR_COMPRESS_INVALID_LEVEL\n\
         ERR_COMPRESS_CHECKSUM_MISMATCH\n\
         ERR_COMPRESS_INVALID_OPTION\n"
    );
}

#[test]
fn decompression_bombs_stop_at_the_default_limits_and_options_raise_them() {
    let stdout = stdout_of(
        r#"import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, decompress } from "tyrus:compress";

const main = async (): Promise<void> => {
    const bomb = await compress("a".repeat(8_000_000), CompressFormat.Gzip, { level: 9 }) onfail {
        print(bomb.code);
    };
    match (await decompress(bomb)) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
    const small: DecompressOptions = { maxOutputBytes: 1_000_000, maxRatio: 1_000_000 };
    match (await decompress(bomb, small)) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
    const generous: DecompressOptions = { maxRatio: 1_000_000 };
    match (await decompress(bomb, generous)) {
        Ok(bytes) => { print(bytes.length); }
        Err(error) => { print(error.code); }
    }
};
"#,
    );
    assert_eq!(
        stdout,
        "ERR_COMPRESS_RATIO_LIMIT\nERR_COMPRESS_OUTPUT_LIMIT\n8000000\n"
    );
}

#[test]
fn an_aliased_format_enum_reaches_the_runtime() {
    let stdout = stdout_of(
        r#"import { CompressError, CompressFormat as Format, CompressOptions, DecompressOptions as Options, compress, decompress } from "tyrus:compress";

const main = async (): Promise<void> => {
    const format: Format = Format.Brotli;
    const packed = await compress("alias", format, { level: 3 }) onfail {
        print(packed.code);
    };
    const options: Options = { format: Format.Brotli };
    const restored = await decompress(packed, options) onfail {
        print(restored.code);
    };
    print(restored.length);
};
"#,
    );
    assert_eq!(stdout, "5\n");
}

#[test]
fn a_literal_level_outside_the_format_range_is_refused_at_the_level() {
    let project = CompressProject::new(
        r#"import { CompressError, CompressFormat as Format, CompressOptions, compress, compressionStream } from "tyrus:compress";

const main = async (): Promise<void> => {
    const fine = await compress("x", Format.Brotli, { level: 11 });
    const refused = await compress("x", Format.Gzip, { level: 10 });
    const stream = compressionStream(Format.Deflate, { level: 10 });
};
"#,
    );
    let check = Command::new(env!("CARGO_BIN_EXE_tyrus"))
        .arg("check")
        .current_dir(&project.root)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&check.stderr);
    assert!(!check.status.success(), "{stderr}");
    assert_eq!(stderr.matches("error[").count(), 2, "{stderr}");
    assert!(
        stderr.contains("error[E3312]: compression level `10` is outside the range `0` to `9` of `Format.Deflate`"),
        "{stderr}"
    );
    assert!(
        stderr.contains(":6:63"),
        "the stream level is anchored too: {stderr}"
    );
    assert!(
        stderr.contains(
            "error[E3312]: compression level `10` is outside the range `0` to `9` of `Format.Gzip`"
        ),
        "{stderr}"
    );
    assert!(
        stderr.contains(":5:63"),
        "the range starts at the level: {stderr}"
    );
    assert!(
        stderr.contains(
            "help: use a `level` from `0` (fastest) to `9` (smallest output) for `Format.Gzip`"
        ),
        "{stderr}"
    );
}

#[test]
fn files_compress_and_decompress_through_streams() {
    let project = CompressProject::new(
        r#"import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compressionStream, decompressionStream } from "tyrus:compress";
import { FsError, readFileStream, writeFileStream } from "tyrus:fs";
import { ReadableStream, StreamError, TransformStream, WritableStream } from "tyrus:stream";

const pipeThrough = async (source: string, destination: string, transform: TransformStream<u8[], u8[]>): Promise<Result<void, FsError>> => {
    const input: ReadableStream<u8[]> = readFileStream(source)?;
    const output: WritableStream<u8[]> = writeFileStream(destination)?;
    try {
        const feeding = input.pipeTo(transform.writable);
        const draining = transform.readable.pipeTo(output);
        await feeding;
        await draining;
        print("done", destination);
    } catch (error: Error) {
        print("failed", destination, error.message);
    }
    return Ok();
};

const reportPipe = async (source: string, destination: string, transform: TransformStream<u8[], u8[]>): Promise<void> => {
    const result = await pipeThrough(source, destination, transform);
    match (result) {
        Ok() => {}
        Err(error) => { print("failed", destination, error.message); }
    }
};

const readFirst = async (transform: TransformStream<u8[], u8[]>): Promise<void> => {
    try {
        const chunk: u8[]? = await transform.readable.read();
        print((chunk ?? []).length);
    } catch (error: CompressError) {
        print(error.code);
    }
};

const main = async (): Promise<void> => {
    await reportPipe("./input.txt", "./input.txt.gz", compressionStream(CompressFormat.Gzip, { level: 9 }));
    await reportPipe("./input.txt.gz", "./restored.txt", decompressionStream());
    const brotli: DecompressOptions = { format: CompressFormat.Brotli };
    await reportPipe("./input.txt", "./input.txt.br", compressionStream(CompressFormat.Brotli));
    await reportPipe("./input.txt.br", "./restored-br.txt", decompressionStream(brotli));
    await reportPipe("./input.txt", "./refused.txt", decompressionStream());
    const level: i32 = 12;
    await readFirst(compressionStream(CompressFormat.Brotli, { level: level }));
};
"#,
    );
    let input: String = (0..50_000)
        .map(|index| format!("streamed line {index}\n"))
        .collect();
    fs::write(project.root.join("input.txt"), &input).unwrap();
    let output = project.build_and_run();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout,
        "done ./input.txt.gz\n\
         done ./restored.txt\n\
         done ./input.txt.br\n\
         done ./restored-br.txt\n\
         failed ./refused.txt the format cannot be detected from the data; pass `format` in the options\n\
         ERR_COMPRESS_INVALID_LEVEL\n"
    );
    for restored in ["restored.txt", "restored-br.txt"] {
        assert_eq!(
            fs::read_to_string(project.root.join(restored)).unwrap(),
            input,
            "{restored}"
        );
    }
    let gzip = fs::read(project.root.join("input.txt.gz")).unwrap();
    assert_eq!(&gzip[..2], &[0x1f, 0x8b]);
}
