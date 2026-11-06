//! `tyrus:compress`: Gzip, Deflate, raw Deflate and Brotli over buffers, with
//! the decompression-bomb limits on by default. Every function fulfils its
//! promise with a `Result<u8[], CompressError>`; none rejects.

#[path = "ffi/ffi.rs"]
mod ffi;
#[path = "streams/streams.rs"]
pub(crate) mod streams;

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

pub use ffi::{
    tyrus_compress_compress, tyrus_compress_compression_stream, tyrus_compress_decompress,
    tyrus_compress_decompression_stream,
};

use crate::codec::{CodecError, Format, Limits};

pub(crate) const ERR_INVALID_DATA: &str = "ERR_COMPRESS_INVALID_DATA";
pub(crate) const ERR_CHECKSUM_MISMATCH: &str = "ERR_COMPRESS_CHECKSUM_MISMATCH";
pub(crate) const ERR_UNKNOWN_FORMAT: &str = "ERR_COMPRESS_UNKNOWN_FORMAT";
pub(crate) const ERR_INVALID_LEVEL: &str = "ERR_COMPRESS_INVALID_LEVEL";
pub(crate) const ERR_INVALID_OPTION: &str = "ERR_COMPRESS_INVALID_OPTION";
pub(crate) const ERR_OUTPUT_LIMIT: &str = "ERR_COMPRESS_OUTPUT_LIMIT";
pub(crate) const ERR_RATIO_LIMIT: &str = "ERR_COMPRESS_RATIO_LIMIT";
pub(crate) const ERR_ALLOCATION_FAILED: &str = "ERR_COMPRESS_ALLOCATION_FAILED";

/// A whole buffer is held in memory, so the default stays well under what a
/// stream may produce.
pub(crate) const DEFAULT_MAX_OUTPUT_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_RATIO: u32 = 200;
/// A stream never holds its whole output, so it may produce far more than a
/// buffer before the limit refuses it.
pub(crate) const DEFAULT_STREAM_MAX_OUTPUT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
/// Bytes each side of a compression stream buffers before a write waits.
pub(crate) const STREAM_CAPACITY_BYTES: usize = 64 * 1024;

pub(crate) fn message_for(code: &str) -> &'static str {
    match code {
        ERR_INVALID_DATA => "the data is malformed, truncated, or not in the given format",
        ERR_CHECKSUM_MISMATCH => "the data does not match its checksum",
        ERR_UNKNOWN_FORMAT => {
            "the format cannot be detected from the data; pass `format` in the options"
        }
        ERR_INVALID_LEVEL => "the compression level is outside the range of the format",
        ERR_INVALID_OPTION => "an option has an invalid value",
        ERR_OUTPUT_LIMIT => "the decompressed output would exceed `maxOutputBytes`",
        ERR_RATIO_LIMIT => "the decompressed output would exceed `maxRatio` times its input",
        ERR_ALLOCATION_FAILED => "not enough memory to hold the output",
        _ => "compression failed",
    }
}

pub(crate) fn code_for(error: CodecError) -> &'static str {
    match error {
        CodecError::InvalidData => ERR_INVALID_DATA,
        CodecError::ChecksumMismatch => ERR_CHECKSUM_MISMATCH,
        CodecError::UnknownFormat => ERR_UNKNOWN_FORMAT,
        CodecError::InvalidLevel => ERR_INVALID_LEVEL,
        CodecError::OutputLimit => ERR_OUTPUT_LIMIT,
        CodecError::RatioLimit => ERR_RATIO_LIMIT,
        CodecError::AllocationFailed => ERR_ALLOCATION_FAILED,
    }
}

pub(crate) type CompressResult<T> = Result<T, &'static str>;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DecompressOptions {
    pub format: Option<i32>,
    pub max_output_bytes: Option<u64>,
    pub max_ratio: Option<u32>,
}

/// Options only raise limits: a `0` would switch a protection off, so it is
/// refused instead.
fn limits(options: DecompressOptions, default_max_output: u64) -> CompressResult<Limits> {
    let max_output = options.max_output_bytes.unwrap_or(default_max_output);
    let max_ratio = options.max_ratio.unwrap_or(DEFAULT_MAX_RATIO);
    if max_output == 0 || max_ratio == 0 {
        return Err(ERR_INVALID_OPTION);
    }
    Ok(Limits {
        max_output,
        max_ratio,
    })
}

pub(crate) fn compress(data: &[u8], format: i32, level: Option<i32>) -> CompressResult<Vec<u8>> {
    let format = Format::from_ordinal(format).ok_or(ERR_INVALID_OPTION)?;
    crate::codec::compress(data, format, level).map_err(code_for)
}

pub(crate) fn decompress(data: &[u8], options: DecompressOptions) -> CompressResult<Vec<u8>> {
    let format = format_option(options.format)?;
    crate::codec::decompress(data, format, limits(options, DEFAULT_MAX_OUTPUT_BYTES)?)
        .map_err(code_for)
}

fn format_option(ordinal: Option<i32>) -> CompressResult<Option<Format>> {
    match ordinal {
        Some(ordinal) => Ok(Some(
            Format::from_ordinal(ordinal).ok_or(ERR_INVALID_OPTION)?,
        )),
        None => Ok(None),
    }
}

pub(crate) fn compression_transform(
    format: i32,
    level: Option<i32>,
) -> CompressResult<streams::EncoderTransform> {
    let format = Format::from_ordinal(format).ok_or(ERR_INVALID_OPTION)?;
    let encoder = crate::codec::Encoder::new(format, level).map_err(code_for)?;
    Ok(streams::EncoderTransform(encoder))
}

pub(crate) fn decompression_transform(
    options: DecompressOptions,
) -> CompressResult<streams::DecoderTransform> {
    let format = format_option(options.format)?;
    let limits = limits(options, DEFAULT_STREAM_MAX_OUTPUT_BYTES)?;
    Ok(streams::DecoderTransform(crate::codec::Decoder::new(
        format, limits,
    )))
}
