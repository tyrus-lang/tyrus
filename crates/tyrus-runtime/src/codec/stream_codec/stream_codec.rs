//! Incremental codecs for the compression streams. Input arrives in chunks
//! and output leaves in pieces of at most `CHUNK` bytes, so memory stays
//! bounded however far the data expands: a decoder never produces more than
//! one piece before the stream's backpressure is consulted again.

use super::deflate::{
    Adler32, GZIP_MAGIC, GZIP_TRAILER, ZLIB_TRAILER, gzip_header_length, zlib_header,
};
use super::{CHUNK, CodecError, DEFAULT_LEVEL, Format, Limits, detect, exceeds_ratio};
use brotli::{BrotliDecompressStream, BrotliResult, BrotliState, HeapAlloc, HuffmanCode};
use flate2::{Decompress, FlushDecompress, Status};
use std::io::Write;

#[cfg(test)]
#[path = "tests/tests.rs"]
mod tests;

/// A header longer than this is refused instead of buffered without end: a
/// Gzip name or comment has no length field of its own.
const MAX_HEADER_BYTES: usize = 64 * 1024;

enum EncoderKind {
    Gzip(flate2::write::GzEncoder<Vec<u8>>),
    Zlib(flate2::write::ZlibEncoder<Vec<u8>>),
    Raw(flate2::write::DeflateEncoder<Vec<u8>>),
    Brotli(Option<Box<brotli::CompressorWriter<Vec<u8>>>>),
}

pub(crate) struct Encoder {
    kind: EncoderKind,
    pending: Vec<u8>,
    offset: usize,
    flush_each_push: bool,
}

impl Encoder {
    pub(crate) fn new(format: Format, level: Option<i32>) -> Result<Self, CodecError> {
        let level = level.unwrap_or(DEFAULT_LEVEL);
        if !format.levels().contains(&level) {
            return Err(CodecError::InvalidLevel);
        }
        #[allow(clippy::cast_sign_loss)]
        let unsigned = level as u32;
        let compression = flate2::Compression::new(unsigned);
        let kind = match format {
            Format::Gzip => EncoderKind::Gzip(
                flate2::GzBuilder::new()
                    .mtime(0)
                    .write(Vec::new(), compression),
            ),
            Format::Deflate => {
                EncoderKind::Zlib(flate2::write::ZlibEncoder::new(Vec::new(), compression))
            }
            Format::DeflateRaw => {
                EncoderKind::Raw(flate2::write::DeflateEncoder::new(Vec::new(), compression))
            }
            Format::Brotli => EncoderKind::Brotli(Some(Box::new(brotli::CompressorWriter::new(
                Vec::new(),
                4096,
                unsigned,
                super::brotli_codec::WINDOW_BITS,
            )))),
        };
        Ok(Self {
            kind,
            pending: Vec::new(),
            offset: 0,
            flush_each_push: false,
        })
    }

    /// Emits everything each pushed chunk can produce right away, at some cost
    /// in ratio, so a streamed response reaches its client chunk by chunk.
    pub(crate) fn flushing(mut self) -> Self {
        self.flush_each_push = true;
        self
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<(), CodecError> {
        let flush = self.flush_each_push;
        let written = match &mut self.kind {
            EncoderKind::Gzip(encoder) => write_then_flush(encoder, chunk, flush),
            EncoderKind::Zlib(encoder) => write_then_flush(encoder, chunk, flush),
            EncoderKind::Raw(encoder) => write_then_flush(encoder, chunk, flush),
            EncoderKind::Brotli(Some(encoder)) => write_then_flush(encoder, chunk, flush),
            EncoderKind::Brotli(None) => return Err(CodecError::InvalidData),
        };
        written.map_err(|_| CodecError::AllocationFailed)?;
        self.collect();
        Ok(())
    }

    pub(crate) fn finish(&mut self) -> Result<(), CodecError> {
        let finished = match &mut self.kind {
            EncoderKind::Gzip(encoder) => encoder.try_finish(),
            EncoderKind::Zlib(encoder) => encoder.try_finish(),
            EncoderKind::Raw(encoder) => encoder.try_finish(),
            EncoderKind::Brotli(encoder) => {
                if let Some(encoder) = encoder.take() {
                    let tail = encoder.into_inner();
                    self.pending.extend_from_slice(&tail);
                }
                Ok(())
            }
        };
        finished.map_err(|_| CodecError::AllocationFailed)?;
        self.collect();
        Ok(())
    }

    /// Moves what the encoder has produced so far into `pending`.
    fn collect(&mut self) {
        let produced = match &mut self.kind {
            EncoderKind::Gzip(encoder) => std::mem::take(encoder.get_mut()),
            EncoderKind::Zlib(encoder) => std::mem::take(encoder.get_mut()),
            EncoderKind::Raw(encoder) => std::mem::take(encoder.get_mut()),
            EncoderKind::Brotli(Some(encoder)) => std::mem::take(encoder.get_mut()),
            EncoderKind::Brotli(None) => Vec::new(),
        };
        if self.offset == self.pending.len() {
            self.pending = produced;
            self.offset = 0;
        } else {
            self.pending.extend_from_slice(&produced);
        }
    }

    pub(crate) fn pull(&mut self) -> Option<Vec<u8>> {
        take_piece(&mut self.pending, &mut self.offset)
    }
}

fn write_then_flush(writer: &mut impl Write, chunk: &[u8], flush: bool) -> std::io::Result<()> {
    writer.write_all(chunk)?;
    if flush {
        writer.flush()?;
    }
    Ok(())
}

fn take_piece(pending: &mut Vec<u8>, offset: &mut usize) -> Option<Vec<u8>> {
    if *offset == pending.len() {
        pending.clear();
        *offset = 0;
        return None;
    }
    let end = (*offset + CHUNK).min(pending.len());
    let piece = pending[*offset..end].to_vec();
    *offset = end;
    Some(piece)
}

enum Stage {
    Detect,
    GzipHeader,
    GzipBody(Box<Decompress>, flate2::Crc),
    GzipTrailer(flate2::Crc),
    GzipBetween,
    ZlibHeader,
    ZlibBody(Box<Decompress>, Adler32),
    ZlibTrailer(Adler32),
    RawBody(Box<Decompress>),
    Brotli(Box<BrotliState<HeapAlloc<u8>, HeapAlloc<u32>, HeapAlloc<HuffmanCode>>>),
    Done,
}

pub(crate) struct Decoder {
    format: Option<Format>,
    stage: Stage,
    input: Vec<u8>,
    ended: bool,
    produced: u64,
    consumed: u64,
    limits: Limits,
}

enum Step {
    /// Output was produced, or the stage moved on.
    Progress(Vec<u8>),
    /// Nothing more until the next input chunk.
    Starved,
}

impl Decoder {
    pub(crate) fn new(format: Option<Format>, limits: Limits) -> Self {
        Self {
            format,
            stage: Stage::Detect,
            input: Vec::new(),
            ended: false,
            produced: 0,
            consumed: 0,
            limits,
        }
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) {
        self.input.extend_from_slice(chunk);
    }

    pub(crate) fn finish(&mut self) {
        self.ended = true;
    }

    /// The next piece of output, `None` when more input is needed or the data
    /// has ended cleanly. Refuses what the one-shot decoder refuses.
    pub(crate) fn pull(&mut self) -> Result<Option<Vec<u8>>, CodecError> {
        loop {
            match self.step()? {
                Step::Progress(output) if !output.is_empty() => {
                    self.produced += output.len() as u64;
                    if self.produced > self.limits.max_output {
                        return Err(CodecError::OutputLimit);
                    }
                    if exceeds_ratio(self.produced, self.consumed, self.limits.max_ratio) {
                        return Err(CodecError::RatioLimit);
                    }
                    return Ok(Some(output));
                }
                Step::Progress(_) => {}
                Step::Starved if self.ended => return self.end_of_input().map(|()| None),
                Step::Starved => return Ok(None),
            }
        }
    }

    fn end_of_input(&self) -> Result<(), CodecError> {
        match self.stage {
            Stage::Done | Stage::GzipBetween => Ok(()),
            Stage::Detect if self.format.is_none() && self.input.len() < 2 => {
                Err(CodecError::UnknownFormat)
            }
            _ => Err(CodecError::InvalidData),
        }
    }

    fn consume(&mut self, count: usize) {
        self.input.drain(..count);
        self.consumed += count as u64;
    }

    fn step(&mut self) -> Result<Step, CodecError> {
        let stage = std::mem::replace(&mut self.stage, Stage::Done);
        let (stage, step) = match stage {
            Stage::Detect => {
                let format = match self.format {
                    Some(format) => format,
                    None if self.input.len() < 2 && !self.ended => {
                        return self.stay(Stage::Detect);
                    }
                    None => detect(&self.input).ok_or(CodecError::UnknownFormat)?,
                };
                let next = match format {
                    Format::Gzip => Stage::GzipHeader,
                    Format::Deflate => Stage::ZlibHeader,
                    Format::DeflateRaw => Stage::RawBody(Box::new(Decompress::new(false))),
                    Format::Brotli => Stage::Brotli(Box::new(BrotliState::new_strict(
                        HeapAlloc::<u8>::new(0),
                        HeapAlloc::<u32>::new(0),
                        HeapAlloc::<HuffmanCode>::new(HuffmanCode { bits: 2, value: 1 }),
                    ))),
                };
                (next, Step::Progress(Vec::new()))
            }
            Stage::GzipHeader => match gzip_header_length(&self.input)? {
                Some(length) => {
                    self.consume(length);
                    (
                        Stage::GzipBody(Box::new(Decompress::new(false)), flate2::Crc::new()),
                        Step::Progress(Vec::new()),
                    )
                }
                None if self.input.len() > MAX_HEADER_BYTES => return Err(CodecError::InvalidData),
                None => return self.stay(Stage::GzipHeader),
            },
            Stage::GzipBody(mut state, mut crc) => {
                let (output, ended, used) = self.inflate(&mut state)?;
                crc.update(&output);
                let next = if ended {
                    Stage::GzipTrailer(crc)
                } else {
                    Stage::GzipBody(state, crc)
                };
                self.starve_or(next, (output, ended, used))?
            }
            Stage::GzipTrailer(crc) => {
                if self.input.len() < GZIP_TRAILER {
                    return self.stay(Stage::GzipTrailer(crc));
                }
                let trailer = &self.input[..GZIP_TRAILER];
                let expected = u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
                let size = u32::from_le_bytes([trailer[4], trailer[5], trailer[6], trailer[7]]);
                if crc.sum() != expected || crc.amount() != size {
                    return Err(CodecError::ChecksumMismatch);
                }
                self.consume(GZIP_TRAILER);
                (Stage::GzipBetween, Step::Progress(Vec::new()))
            }
            Stage::GzipBetween => match self.input.as_slice() {
                [] => return self.stay(Stage::GzipBetween),
                [first] if *first == GZIP_MAGIC[0] => return self.stay(Stage::GzipBetween),
                [first, second, ..] if [*first, *second] == GZIP_MAGIC => {
                    (Stage::GzipHeader, Step::Progress(Vec::new()))
                }
                _ => return Err(CodecError::InvalidData),
            },
            Stage::ZlibHeader => match zlib_header(&self.input)? {
                Some(()) => {
                    self.consume(2);
                    (
                        Stage::ZlibBody(Box::new(Decompress::new(false)), Adler32::new()),
                        Step::Progress(Vec::new()),
                    )
                }
                None => return self.stay(Stage::ZlibHeader),
            },
            Stage::ZlibBody(mut state, mut adler) => {
                let (output, ended, used) = self.inflate(&mut state)?;
                adler.update(&output);
                let next = if ended {
                    Stage::ZlibTrailer(adler)
                } else {
                    Stage::ZlibBody(state, adler)
                };
                self.starve_or(next, (output, ended, used))?
            }
            Stage::ZlibTrailer(adler) => {
                if self.input.len() < ZLIB_TRAILER {
                    return self.stay(Stage::ZlibTrailer(adler));
                }
                let trailer = &self.input[..ZLIB_TRAILER];
                let expected = u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]);
                if adler.sum() != expected {
                    return Err(CodecError::ChecksumMismatch);
                }
                self.consume(ZLIB_TRAILER);
                (Stage::Done, Step::Progress(Vec::new()))
            }
            Stage::RawBody(mut state) => {
                let (output, ended, used) = self.inflate(&mut state)?;
                let next = if ended {
                    Stage::Done
                } else {
                    Stage::RawBody(state)
                };
                self.starve_or(next, (output, ended, used))?
            }
            Stage::Brotli(mut state) => {
                let (output, ended, used) = self.brotli(&mut state)?;
                let next = if ended {
                    Stage::Done
                } else {
                    Stage::Brotli(state)
                };
                self.starve_or(next, (output, ended, used))?
            }
            Stage::Done => {
                if !self.input.is_empty() {
                    return Err(CodecError::InvalidData);
                }
                return self.stay(Stage::Done);
            }
        };
        self.stage = stage;
        Ok(step)
    }

    fn stay(&mut self, stage: Stage) -> Result<Step, CodecError> {
        self.stage = stage;
        Ok(Step::Starved)
    }

    /// A body stage that neither consumed nor produced anything and did not
    /// end is waiting for input.
    fn starve_or(
        &self,
        next: Stage,
        (output, ended, used): (Vec<u8>, bool, usize),
    ) -> Result<(Stage, Step), CodecError> {
        if output.is_empty() && !ended && used == 0 {
            return Ok((next, Step::Starved));
        }
        Ok((next, Step::Progress(output)))
    }

    /// Inflates up to one piece from the buffered input. `true` once the raw
    /// stream has reached its final block.
    fn inflate(&mut self, state: &mut Decompress) -> Result<(Vec<u8>, bool, usize), CodecError> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(CHUNK)
            .map_err(|_| CodecError::AllocationFailed)?;
        let before = state.total_in();
        let status = state
            .decompress_vec(&self.input, &mut output, FlushDecompress::None)
            .map_err(|_| CodecError::InvalidData)?;
        let used =
            usize::try_from(state.total_in() - before).map_err(|_| CodecError::InvalidData)?;
        self.consume(used);
        Ok((output, matches!(status, Status::StreamEnd), used))
    }

    fn brotli(
        &mut self,
        state: &mut BrotliState<HeapAlloc<u8>, HeapAlloc<u32>, HeapAlloc<HuffmanCode>>,
    ) -> Result<(Vec<u8>, bool, usize), CodecError> {
        let mut buffer = vec![0_u8; CHUNK];
        let mut available_in = self.input.len();
        let mut input_offset = 0;
        let mut available_out = CHUNK;
        let mut output_offset = 0;
        let mut total_out = 0;
        let result = BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            &self.input,
            &mut available_out,
            &mut output_offset,
            &mut buffer,
            &mut total_out,
            state,
        );
        self.consume(input_offset);
        buffer.truncate(output_offset);
        match result {
            BrotliResult::ResultSuccess => Ok((buffer, true, input_offset)),
            BrotliResult::NeedsMoreInput | BrotliResult::NeedsMoreOutput => {
                Ok((buffer, false, input_offset))
            }
            BrotliResult::ResultFailure => Err(CodecError::InvalidData),
        }
    }
}
