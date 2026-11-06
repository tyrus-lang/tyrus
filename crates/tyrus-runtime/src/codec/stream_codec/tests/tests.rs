use super::*;
use crate::codec::{compress, decompress};

const LIMITS: Limits = Limits {
    max_output: 256 * 1024 * 1024,
    max_ratio: 200,
};

const ALL: [Format; 4] = [
    Format::Gzip,
    Format::Deflate,
    Format::DeflateRaw,
    Format::Brotli,
];

fn sample() -> Vec<u8> {
    (0..20_000_u32)
        .flat_map(|index| format!("line {index} of tyrus\n").into_bytes())
        .collect()
}

fn encode_in_pieces(data: &[u8], format: Format, piece: usize) -> Vec<u8> {
    let mut encoder = Encoder::new(format, None).unwrap();
    let mut output = Vec::new();
    for chunk in data.chunks(piece.max(1)) {
        encoder.push(chunk).unwrap();
        while let Some(bytes) = encoder.pull() {
            assert!(bytes.len() <= CHUNK);
            output.extend(bytes);
        }
    }
    encoder.finish().unwrap();
    while let Some(bytes) = encoder.pull() {
        assert!(bytes.len() <= CHUNK);
        output.extend(bytes);
    }
    output
}

fn decode_in_pieces(
    data: &[u8],
    format: Option<Format>,
    limits: Limits,
    piece: usize,
) -> Result<Vec<u8>, CodecError> {
    let mut decoder = Decoder::new(format, limits);
    let mut output = Vec::new();
    let drain = |decoder: &mut Decoder, output: &mut Vec<u8>| -> Result<(), CodecError> {
        while let Some(bytes) = decoder.pull()? {
            assert!(bytes.len() <= CHUNK, "a piece never exceeds one chunk");
            output.extend(bytes);
        }
        Ok(())
    };
    for chunk in data.chunks(piece.max(1)) {
        decoder.push(chunk);
        drain(&mut decoder, &mut output)?;
    }
    decoder.finish();
    drain(&mut decoder, &mut output)?;
    Ok(output)
}

#[test]
fn pieces_of_any_size_encode_and_decode_like_one_buffer() {
    let data = sample();
    for format in ALL {
        for piece in [1, 7, 4096, 64 * 1024, data.len()] {
            let streamed = encode_in_pieces(&data, format, piece);
            assert_eq!(
                decompress(&streamed, Some(format), LIMITS).unwrap(),
                data,
                "{format:?} encoded in {piece}-byte pieces"
            );
            let whole = compress(&data, format, None).unwrap();
            assert_eq!(
                decode_in_pieces(&whole, Some(format), LIMITS, piece).unwrap(),
                data,
                "{format:?} decoded in {piece}-byte pieces"
            );
        }
    }
}

#[test]
fn empty_input_encodes_and_decodes() {
    for format in ALL {
        let streamed = encode_in_pieces(b"", format, 1);
        assert_eq!(
            decode_in_pieces(&streamed, Some(format), LIMITS, 1).unwrap(),
            b""
        );
    }
}

#[test]
fn detection_and_concatenated_members_work_across_piece_boundaries() {
    let mut data = compress(b"first ", Format::Gzip, None).unwrap();
    data.extend(compress(b"", Format::Gzip, None).unwrap());
    data.extend(compress(b"second", Format::Gzip, None).unwrap());
    for piece in [1, 2, 3, 1024] {
        assert_eq!(
            decode_in_pieces(&data, None, LIMITS, piece).unwrap(),
            b"first second"
        );
    }
    let zlib = compress(b"zlib", Format::Deflate, None).unwrap();
    assert_eq!(decode_in_pieces(&zlib, None, LIMITS, 1).unwrap(), b"zlib");
    let brotli = compress(b"x", Format::Brotli, None).unwrap();
    assert_eq!(
        decode_in_pieces(&brotli, None, LIMITS, 1),
        Err(CodecError::UnknownFormat)
    );
    assert_eq!(
        decode_in_pieces(b"\x1f", None, LIMITS, 1),
        Err(CodecError::UnknownFormat)
    );
}

#[test]
fn the_incremental_decoder_refuses_what_the_buffer_decoder_refuses() {
    for format in ALL {
        let packed = compress(&sample(), format, None).unwrap();
        let mut trailing = packed.clone();
        trailing.extend_from_slice(b"garbage");
        let truncated = &packed[..packed.len() / 2];
        for piece in [1, 4096] {
            assert_eq!(
                decode_in_pieces(&trailing, Some(format), LIMITS, piece),
                Err(CodecError::InvalidData),
                "{format:?} trailing, {piece}"
            );
            assert_eq!(
                decode_in_pieces(truncated, Some(format), LIMITS, piece),
                Err(CodecError::InvalidData),
                "{format:?} truncated, {piece}"
            );
        }
    }
    let mut gzip = compress(b"checksummed", Format::Gzip, None).unwrap();
    let crc = gzip.len() - 8;
    gzip[crc] ^= 0xff;
    assert_eq!(
        decode_in_pieces(&gzip, None, LIMITS, 3),
        Err(CodecError::ChecksumMismatch)
    );
    let mut zlib = compress(b"checksummed", Format::Deflate, None).unwrap();
    let last = zlib.len() - 1;
    zlib[last] ^= 0xff;
    assert_eq!(
        decode_in_pieces(&zlib, None, LIMITS, 3),
        Err(CodecError::ChecksumMismatch)
    );
}

#[test]
fn a_bomb_stops_at_the_limit_one_piece_at_a_time() {
    let zeros = vec![0_u8; 16 * 1024 * 1024];
    for format in ALL {
        let bomb = compress(&zeros, format, Some(9)).unwrap();
        assert_eq!(
            decode_in_pieces(&bomb, Some(format), LIMITS, 64 * 1024),
            Err(CodecError::RatioLimit),
            "{format:?}"
        );
        let capped = Limits {
            max_output: 1024 * 1024,
            max_ratio: u32::MAX,
        };
        assert_eq!(
            decode_in_pieces(&bomb, Some(format), capped, 64 * 1024),
            Err(CodecError::OutputLimit),
            "{format:?}"
        );
    }
}

#[test]
fn a_gzip_header_that_never_ends_is_refused_instead_of_buffered() {
    let mut header = vec![0x1f, 0x8b, 8, 0x08, 0, 0, 0, 0, 0, 255];
    header.extend(std::iter::repeat_n(b'a', MAX_HEADER_BYTES + 1));
    assert_eq!(
        decode_in_pieces(&header, None, LIMITS, 4096),
        Err(CodecError::InvalidData)
    );
}

#[test]
fn an_invalid_level_is_refused_before_any_input() {
    assert!(matches!(
        Encoder::new(Format::Brotli, Some(12)),
        Err(CodecError::InvalidLevel)
    ));
}
