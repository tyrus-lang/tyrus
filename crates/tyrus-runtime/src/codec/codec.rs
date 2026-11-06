//! Byte-stream codecs shared by `tyrus:compress`, `tyrus:archive` and the HTTP
//! response compression. Decoding frames Gzip and zlib here, over a raw
//! inflater, so every failure maps to one exact reason and the compressed
//! bytes consumed are known for the ratio limit.

#[path = "brotli_codec/brotli_codec.rs"]
mod brotli_codec;
#[path = "deflate/deflate.rs"]
mod deflate;
#[path = "io_adapters/io_adapters.rs"]
mod io_adapters;
#[path = "stream_codec/stream_codec.rs"]
mod stream_codec;

pub(crate) use io_adapters::{DecoderReader, EncoderWriter};
pub(crate) use stream_codec::{Decoder, Encoder};

/// No output or ratio limit, for a caller that enforces its own.
pub(crate) const UNLIMITED: Limits = Limits {
    max_output: u64::MAX,
    max_ratio: u32::MAX,
};

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "interop_tests/interop_tests.rs"]
mod interop_tests;

use std::io::Write;

/// Past this many bytes produced, the ratio limit applies. Small inputs expand
/// by large factors legitimately (a few bytes of zeros), and refusing them
/// would reject ordinary data long before any real bomb.
pub(crate) const RATIO_FLOOR_BYTES: u64 = 1024 * 1024;

/// Output is produced in chunks of this size, so no limit is passed by more
/// than one chunk before it is noticed.
const CHUNK: usize = 64 * 1024;

/// Ordinals are part of the ABI: the compiler lowers `CompressFormat` to them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Gzip,
    Deflate,
    DeflateRaw,
    Brotli,
}

impl Format {
    pub(crate) fn from_ordinal(ordinal: i32) -> Option<Self> {
        match ordinal {
            0 => Some(Self::Gzip),
            1 => Some(Self::Deflate),
            2 => Some(Self::DeflateRaw),
            3 => Some(Self::Brotli),
            _ => None,
        }
    }

    pub(crate) fn levels(self) -> std::ops::RangeInclusive<i32> {
        match self {
            Self::Gzip | Self::Deflate | Self::DeflateRaw => 0..=9,
            Self::Brotli => 0..=11,
        }
    }
}

pub(crate) const DEFAULT_LEVEL: i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodecError {
    InvalidData,
    ChecksumMismatch,
    UnknownFormat,
    InvalidLevel,
    OutputLimit,
    RatioLimit,
    AllocationFailed,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub max_output: u64,
    pub max_ratio: u32,
}

pub(crate) fn exceeds_ratio(written: u64, compressed: u64, ratio: u32) -> bool {
    written > RATIO_FLOOR_BYTES && written > compressed.saturating_mul(u64::from(ratio))
}

/// The format a buffer announces with its own signature. Brotli and raw
/// Deflate have none, so they are never guessed.
pub(crate) fn detect(data: &[u8]) -> Option<Format> {
    match data {
        [0x1f, 0x8b, ..] => Some(Format::Gzip),
        [cmf, flg, ..]
            if cmf & 0x0f == 8
                && cmf >> 4 <= 7
                && (u16::from(*cmf) * 256 + u16::from(*flg)) % 31 == 0 =>
        {
            Some(Format::Deflate)
        }
        _ => None,
    }
}

/// Same input, format and level always give the same bytes: the Gzip header
/// carries no timestamp and an unknown operating system.
pub(crate) fn compress(
    data: &[u8],
    format: Format,
    level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    let level = level.unwrap_or(DEFAULT_LEVEL);
    if !format.levels().contains(&level) {
        return Err(CodecError::InvalidLevel);
    }
    let mut output = Vec::new();
    output
        .try_reserve(data.len() / 2 + 64)
        .map_err(|_| CodecError::AllocationFailed)?;
    #[allow(clippy::cast_sign_loss)]
    let unsigned = level as u32;
    let written = match format {
        Format::Gzip => {
            let mut encoder = flate2::GzBuilder::new()
                .mtime(0)
                .write(output, flate2::Compression::new(unsigned));
            encoder.write_all(data).and_then(|()| encoder.finish())
        }
        Format::Deflate => {
            let mut encoder =
                flate2::write::ZlibEncoder::new(output, flate2::Compression::new(unsigned));
            encoder.write_all(data).and_then(|()| encoder.finish())
        }
        Format::DeflateRaw => {
            let mut encoder =
                flate2::write::DeflateEncoder::new(output, flate2::Compression::new(unsigned));
            encoder.write_all(data).and_then(|()| encoder.finish())
        }
        Format::Brotli => Ok(brotli_codec::compress(data, output, unsigned)),
    };
    written.map_err(|_| CodecError::AllocationFailed)
}

/// Decodes `data` as `format`, or as the format its signature announces.
/// Output and ratio are checked while producing, never after.
pub(crate) fn decompress(
    data: &[u8],
    format: Option<Format>,
    limits: Limits,
) -> Result<Vec<u8>, CodecError> {
    let format = match format {
        Some(format) => format,
        None => detect(data).ok_or(CodecError::UnknownFormat)?,
    };
    let mut sink = Sink {
        output: Vec::new(),
        limits,
    };
    match format {
        Format::Gzip => deflate::gzip(data, &mut sink)?,
        Format::Deflate => deflate::zlib(data, &mut sink)?,
        Format::DeflateRaw => deflate::raw(data, &mut sink)?,
        Format::Brotli => brotli_codec::decompress(data, &mut sink)?,
    }
    Ok(sink.output)
}

struct Sink {
    output: Vec<u8>,
    limits: Limits,
}

impl Sink {
    /// Makes room for at least one more chunk without letting the capacity
    /// run far past the output limit, which doubling alone would.
    fn reserve_chunk(&mut self) -> Result<(), CodecError> {
        let length = self.output.len();
        if self.output.capacity() - length >= CHUNK {
            return Ok(());
        }
        let ceiling = usize::try_from(self.limits.max_output)
            .unwrap_or(usize::MAX)
            .saturating_add(CHUNK);
        let wanted = length
            .saturating_mul(2)
            .max(length + CHUNK)
            .min(ceiling.max(length + CHUNK));
        self.output
            .try_reserve_exact(wanted - length)
            .map_err(|_| CodecError::AllocationFailed)
    }

    fn check(&self, consumed: u64) -> Result<(), CodecError> {
        let produced = self.output.len() as u64;
        if produced > self.limits.max_output {
            return Err(CodecError::OutputLimit);
        }
        if exceeds_ratio(produced, consumed, self.limits.max_ratio) {
            return Err(CodecError::RatioLimit);
        }
        Ok(())
    }
}

#[cfg(kani)]
mod kani_proofs {
    use super::{RATIO_FLOOR_BYTES, exceeds_ratio};

    /// For every input, including counts near `u64::MAX` where the ceiling
    /// saturates: nothing at or under the floor is refused, and anything
    /// refused really is above `ratio` times the compressed size.
    #[kani::proof]
    fn the_ratio_check_never_refuses_below_the_floor_or_within_the_ratio() {
        let written: u64 = kani::any();
        let compressed: u64 = kani::any();
        let ratio: u32 = kani::any();
        kani::assume(ratio > 0);
        if exceeds_ratio(written, compressed, ratio) {
            assert!(written > RATIO_FLOOR_BYTES);
            assert!(u128::from(written) > u128::from(compressed) * u128::from(ratio));
        } else if written > RATIO_FLOOR_BYTES {
            assert!(u128::from(written) <= u128::from(compressed) * u128::from(ratio));
        }
    }
}
