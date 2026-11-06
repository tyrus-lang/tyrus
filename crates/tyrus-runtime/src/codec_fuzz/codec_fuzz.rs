//! Entry point for the `compress_decompress` fuzz target. The first byte picks
//! a format (or detection) and a level; the rest is decoded with small limits,
//! which must never panic or produce more than they allow, both in one buffer
//! and incrementally in pieces whose size the input also picks, and the two
//! must agree; then it is compressed and decoded back, which must give the
//! same bytes.

use crate::codec::{CodecError, Decoder, Format, Limits, compress, decompress};

const LIMITS: Limits = Limits {
    max_output: 4 * 1024 * 1024,
    max_ratio: 200,
};

const FORMATS: [Format; 4] = [
    Format::Gzip,
    Format::Deflate,
    Format::DeflateRaw,
    Format::Brotli,
];

pub fn compress_and_decompress(data: &[u8]) {
    let Some((&selector, payload)) = data.split_first() else {
        return;
    };
    let format = FORMATS[usize::from(selector) % FORMATS.len()];
    let requested = if selector & 0x80 == 0 {
        Some(format)
    } else {
        None
    };
    let whole = decompress(payload, requested, LIMITS);
    match &whole {
        Ok(output) => assert!(output.len() as u64 <= LIMITS.max_output),
        Err(CodecError::InvalidLevel) => panic!("decoding never reports a level"),
        Err(_) => {}
    }
    let piece = usize::from(selector % 61) + 1;
    let incremental = decode_in_pieces(payload, requested, piece);
    match (&whole, &incremental) {
        (Ok(whole), Ok(pieces)) => assert_eq!(whole, pieces),
        (Err(whole), Err(pieces)) => assert_eq!(whole, pieces),
        // A limit may be crossed at a different moment: each piece of output
        // is checked as it is produced.
        (Err(CodecError::OutputLimit | CodecError::RatioLimit), _)
        | (_, Err(CodecError::OutputLimit | CodecError::RatioLimit)) => {}
        (whole, pieces) => panic!("one buffer gave {whole:?}, pieces gave {pieces:?}"),
    }
    let level = i32::from(selector >> 2) % 10;
    let packed = compress(payload, format, Some(level)).expect("every input compresses");
    let generous = Limits {
        max_output: u64::MAX,
        max_ratio: u32::MAX,
    };
    assert_eq!(
        decompress(&packed, Some(format), generous).expect("own output decodes"),
        payload
    );
}

fn decode_in_pieces(
    payload: &[u8],
    format: Option<Format>,
    piece: usize,
) -> Result<Vec<u8>, CodecError> {
    let mut decoder = Decoder::new(format, LIMITS);
    let mut output = Vec::new();
    for chunk in payload.chunks(piece) {
        decoder.push(chunk);
        while let Some(bytes) = decoder.pull()? {
            output.extend(bytes);
        }
    }
    decoder.finish();
    while let Some(bytes) = decoder.pull()? {
        output.extend(bytes);
    }
    Ok(output)
}
