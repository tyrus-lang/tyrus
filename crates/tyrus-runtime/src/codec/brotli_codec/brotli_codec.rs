use super::{CHUNK, CodecError, Sink};
use brotli::{BrotliDecompressStream, BrotliResult, BrotliState, HeapAlloc, HuffmanCode};
use std::io::Write;

/// Window of 4 MiB, the encoder's usual default; decoders must accept up to
/// 16 MiB, so every standard decoder reads the output.
pub(super) const WINDOW_BITS: u32 = 22;

pub(super) fn compress(data: &[u8], output: Vec<u8>, quality: u32) -> Vec<u8> {
    let mut encoder = brotli::CompressorWriter::new(output, 4096, quality, WINDOW_BITS);
    // Writing into a `Vec` cannot fail short of aborting on allocation.
    let _ = encoder.write_all(data);
    encoder.into_inner()
}

/// Brotli carries no checksum, so the only failures are malformed or
/// truncated data and the limits.
pub(super) fn decompress(input: &[u8], sink: &mut Sink) -> Result<(), CodecError> {
    // `new_strict` keeps the RFC 7932 window; `new` also accepts the
    // non-standard large window.
    let mut state = BrotliState::new_strict(
        HeapAlloc::<u8>::new(0),
        HeapAlloc::<u32>::new(0),
        HeapAlloc::<HuffmanCode>::new(HuffmanCode { bits: 2, value: 1 }),
    );
    let mut buffer = vec![0_u8; CHUNK];
    let mut available_in = input.len();
    let mut input_offset = 0;
    let mut total_out = 0;
    loop {
        let mut available_out = CHUNK;
        let mut output_offset = 0;
        let result = BrotliDecompressStream(
            &mut available_in,
            &mut input_offset,
            input,
            &mut available_out,
            &mut output_offset,
            &mut buffer,
            &mut total_out,
            &mut state,
        );
        sink.reserve_chunk()?;
        sink.output.extend_from_slice(&buffer[..output_offset]);
        sink.check(input_offset as u64)?;
        match result {
            BrotliResult::NeedsMoreOutput => {}
            BrotliResult::ResultSuccess if input_offset == input.len() => return Ok(()),
            BrotliResult::ResultSuccess
            | BrotliResult::NeedsMoreInput
            | BrotliResult::ResultFailure => return Err(CodecError::InvalidData),
        }
    }
}
