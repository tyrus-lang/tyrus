//! Compressed bytes cross the boundary in both directions with the tools
//! people actually use: `gzip`, `pigz` and Python's `gzip`, `zlib` and
//! `brotli`. A tool missing from the machine skips its half instead of failing.

use super::*;
use std::io::Write;
use std::process::{Command, Stdio};

const LIMITS: Limits = Limits {
    max_output: u64::MAX,
    max_ratio: u32::MAX,
};

fn sample() -> Vec<u8> {
    let mut data = b"hello interop, ".repeat(2_000);
    data.extend((0..=255_u8).cycle().take(64 * 1024));
    data.extend("ação é ünïcode\n".as_bytes());
    data
}

/// Runs `program` with `input` on stdin and returns its stdout, or `None` when
/// the tool (or the Python module it needs) is not installed.
fn pipe(program: &str, args: &[&str], input: &[u8]) -> Option<Vec<u8>> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let input = input.to_vec();
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output().ok()?;
    writer.join().expect("stdin writer");
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("No module named") {
            return None;
        }
        panic!("`{program} {args:?}` failed: {stderr}");
    }
    Some(output.stdout)
}

fn python(script: &str, input: &[u8]) -> Option<Vec<u8>> {
    pipe("python3", &["-c", script], input)
}

const PY_GZIP_DECOMPRESS: &str =
    "import gzip, sys; sys.stdout.buffer.write(gzip.decompress(sys.stdin.buffer.read()))";
const PY_GZIP_COMPRESS: &str =
    "import gzip, sys; sys.stdout.buffer.write(gzip.compress(sys.stdin.buffer.read()))";
const PY_ZLIB_DECOMPRESS: &str =
    "import zlib, sys; sys.stdout.buffer.write(zlib.decompress(sys.stdin.buffer.read()))";
const PY_ZLIB_COMPRESS: &str =
    "import zlib, sys; sys.stdout.buffer.write(zlib.compress(sys.stdin.buffer.read(), 9))";
const PY_RAW_DECOMPRESS: &str =
    "import zlib, sys; sys.stdout.buffer.write(zlib.decompress(sys.stdin.buffer.read(), -15))";
const PY_RAW_COMPRESS: &str = "import zlib, sys; c = zlib.compressobj(9, zlib.DEFLATED, -15); \
    sys.stdout.buffer.write(c.compress(sys.stdin.buffer.read()) + c.flush())";
const PY_BROTLI_DECOMPRESS: &str =
    "import brotli, sys; sys.stdout.buffer.write(brotli.decompress(sys.stdin.buffer.read()))";
const PY_BROTLI_COMPRESS: &str = "import brotli, sys; sys.stdout.buffer.write(brotli.compress(sys.stdin.buffer.read(), quality=11))";

#[test]
fn gzip_tools_read_tyrus_output_and_tyrus_reads_theirs() {
    let data = sample();
    let packed = compress(&data, Format::Gzip, Some(9)).unwrap();
    for (program, args) in [("gzip", &["-dc"][..]), ("pigz", &["-dc"][..])] {
        if let Some(output) = pipe(program, args, &packed) {
            assert_eq!(output, data, "{program} reading tyrus");
        }
        if let Some(theirs) = pipe(program, &["-c", "-9"], &data) {
            assert_eq!(
                decompress(&theirs, None, LIMITS).unwrap(),
                data,
                "tyrus reading {program}"
            );
        }
    }
    if let Some(output) = python(PY_GZIP_DECOMPRESS, &packed) {
        assert_eq!(output, data);
    }
    if let Some(theirs) = python(PY_GZIP_COMPRESS, &data) {
        assert_eq!(decompress(&theirs, None, LIMITS).unwrap(), data);
    }
}

#[test]
fn concatenated_members_from_gzip_are_read_like_gzip_reads_them() {
    let (Some(first), Some(second)) = (
        pipe("gzip", &["-c"], b"first half, "),
        pipe("gzip", &["-c"], b"second half"),
    ) else {
        return;
    };
    let joined = [first, second].concat();
    let expected = pipe("gzip", &["-dc"], &joined).unwrap();
    assert_eq!(decompress(&joined, None, LIMITS).unwrap(), expected);
}

#[test]
fn zlib_and_raw_deflate_match_python() {
    let data = sample();
    for (format, decode, encode) in [
        (Format::Deflate, PY_ZLIB_DECOMPRESS, PY_ZLIB_COMPRESS),
        (Format::DeflateRaw, PY_RAW_DECOMPRESS, PY_RAW_COMPRESS),
    ] {
        let packed = compress(&data, format, None).unwrap();
        if let Some(output) = python(decode, &packed) {
            assert_eq!(output, data, "python reading {format:?}");
        }
        if let Some(theirs) = python(encode, &data) {
            assert_eq!(
                decompress(&theirs, Some(format), LIMITS).unwrap(),
                data,
                "tyrus reading python {format:?}"
            );
        }
    }
}

#[test]
fn brotli_matches_the_reference_implementation() {
    let data = sample();
    let packed = compress(&data, Format::Brotli, Some(11)).unwrap();
    if let Some(output) = python(PY_BROTLI_DECOMPRESS, &packed) {
        assert_eq!(output, data);
    }
    if let Some(theirs) = python(PY_BROTLI_COMPRESS, &data) {
        assert_eq!(
            decompress(&theirs, Some(Format::Brotli), LIMITS).unwrap(),
            data
        );
    }
}
