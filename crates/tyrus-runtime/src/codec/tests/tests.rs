use super::deflate::adler32;
use super::*;

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

fn compressed(format: Format, data: &[u8]) -> Vec<u8> {
    compress(data, format, None).expect("compression succeeds")
}

#[test]
fn every_format_round_trips_at_every_level() {
    let data = sample();
    for format in ALL {
        for level in format.levels() {
            let packed = compress(&data, format, Some(level)).unwrap();
            assert_eq!(
                decompress(&packed, Some(format), LIMITS).unwrap(),
                data,
                "{format:?} at level {level}"
            );
        }
    }
}

#[test]
fn empty_input_round_trips() {
    for format in ALL {
        let packed = compressed(format, b"");
        assert_eq!(decompress(&packed, Some(format), LIMITS).unwrap(), b"");
    }
}

#[test]
fn compression_is_deterministic_and_the_gzip_header_carries_no_timestamp() {
    let data = sample();
    for format in ALL {
        assert_eq!(compressed(format, &data), compressed(format, &data));
    }
    let gzip = compressed(Format::Gzip, b"tyrus");
    assert_eq!(&gzip[4..8], &[0, 0, 0, 0], "mtime");
    assert_eq!(gzip[9], 255, "unknown operating system");
}

#[test]
fn signatures_identify_gzip_and_zlib_only() {
    assert_eq!(detect(&compressed(Format::Gzip, b"a")), Some(Format::Gzip));
    assert_eq!(
        detect(&compressed(Format::Deflate, b"a")),
        Some(Format::Deflate)
    );
    assert_eq!(detect(&compressed(Format::DeflateRaw, b"a")), None);
    assert_eq!(detect(&compressed(Format::Brotli, b"a")), None);
    assert_eq!(detect(b""), None);
    for format in [Format::Gzip, Format::Deflate] {
        let packed = compressed(format, b"detected");
        assert_eq!(decompress(&packed, None, LIMITS).unwrap(), b"detected");
    }
    assert_eq!(
        decompress(&compressed(Format::Brotli, b"x"), None, LIMITS),
        Err(CodecError::UnknownFormat)
    );
}

#[test]
fn concatenated_gzip_members_decode_in_order() {
    let mut data = compressed(Format::Gzip, b"first ");
    data.extend(compressed(Format::Gzip, b""));
    data.extend(compressed(Format::Gzip, b"second"));
    assert_eq!(
        decompress(&data, Some(Format::Gzip), LIMITS).unwrap(),
        b"first second"
    );
    let empty = compressed(Format::Gzip, b"");
    let many: Vec<u8> = std::iter::repeat_n(empty, 5_000).flatten().collect();
    assert_eq!(decompress(&many, Some(Format::Gzip), LIMITS).unwrap(), b"");
}

#[test]
fn trailing_bytes_truncation_and_wrong_formats_are_invalid_data() {
    for format in ALL {
        let packed = compressed(format, &sample());
        let mut trailing = packed.clone();
        trailing.extend_from_slice(b"garbage");
        assert_eq!(
            decompress(&trailing, Some(format), LIMITS),
            Err(CodecError::InvalidData),
            "{format:?} with trailing bytes"
        );
        let truncated = &packed[..packed.len() / 2];
        assert_eq!(
            decompress(truncated, Some(format), LIMITS),
            Err(CodecError::InvalidData),
            "{format:?} truncated"
        );
        assert_eq!(
            decompress(b"", Some(format), LIMITS),
            Err(CodecError::InvalidData),
            "{format:?} empty"
        );
    }
    let gzip = compressed(Format::Gzip, b"tyrus");
    for format in [Format::Deflate, Format::DeflateRaw, Format::Brotli] {
        assert!(
            decompress(&gzip, Some(format), LIMITS).is_err(),
            "{format:?}"
        );
    }
}

#[test]
fn a_wrong_checksum_or_size_is_a_checksum_mismatch() {
    let mut gzip = compressed(Format::Gzip, b"checksummed");
    let crc = gzip.len() - 8;
    gzip[crc] ^= 0xff;
    assert_eq!(
        decompress(&gzip, Some(Format::Gzip), LIMITS),
        Err(CodecError::ChecksumMismatch)
    );
    let mut sized = compressed(Format::Gzip, b"checksummed");
    let size = sized.len() - 4;
    sized[size] ^= 0x01;
    assert_eq!(
        decompress(&sized, Some(Format::Gzip), LIMITS),
        Err(CodecError::ChecksumMismatch)
    );
    let mut zlib = compressed(Format::Deflate, b"checksummed");
    let last = zlib.len() - 1;
    zlib[last] ^= 0xff;
    assert_eq!(
        decompress(&zlib, Some(Format::Deflate), LIMITS),
        Err(CodecError::ChecksumMismatch)
    );
}

#[test]
fn optional_gzip_header_fields_are_skipped_and_the_header_crc_checked() {
    let body = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    let deflated = {
        let mut encoder = body;
        encoder.write_all(b"fields").unwrap();
        encoder.finish().unwrap()
    };
    let mut header = vec![0x1f, 0x8b, 8, 0x02 | 0x04 | 0x08 | 0x10, 0, 0, 0, 0, 0, 255];
    header.extend_from_slice(&[3, 0, b'a', b'b', b'c']);
    header.extend_from_slice(b"name.txt\0");
    header.extend_from_slice(b"a comment\0");
    let mut crc = flate2::Crc::new();
    crc.update(&header);
    #[allow(clippy::cast_possible_truncation)]
    let low = crc.sum() as u16;
    let mut member = header.clone();
    member.extend_from_slice(&low.to_le_bytes());
    member.extend_from_slice(&deflated);
    let mut content = flate2::Crc::new();
    content.update(b"fields");
    member.extend_from_slice(&content.sum().to_le_bytes());
    member.extend_from_slice(&6_u32.to_le_bytes());
    assert_eq!(
        decompress(&member, Some(Format::Gzip), LIMITS).unwrap(),
        b"fields"
    );
    let crc_at = header.len();
    member[crc_at] ^= 0xff;
    assert_eq!(
        decompress(&member, Some(Format::Gzip), LIMITS),
        Err(CodecError::ChecksumMismatch)
    );
}

#[test]
fn output_beyond_the_limit_is_refused_while_producing() {
    let zeros = vec![0_u8; 4 * 1024 * 1024];
    for format in ALL {
        let packed = compressed(format, &zeros);
        let limits = Limits {
            max_output: 1024 * 1024,
            max_ratio: u32::MAX,
        };
        assert_eq!(
            decompress(&packed, Some(format), limits),
            Err(CodecError::OutputLimit),
            "{format:?}"
        );
    }
}

#[test]
fn expansion_beyond_the_ratio_is_refused_only_past_the_floor() {
    let zeros = vec![0_u8; 8 * 1024 * 1024];
    for format in ALL {
        let packed = compressed(format, &zeros);
        assert_eq!(
            decompress(&packed, Some(format), LIMITS),
            Err(CodecError::RatioLimit),
            "{format:?}"
        );
        let generous = Limits {
            max_output: u64::MAX,
            max_ratio: u32::MAX,
        };
        assert_eq!(
            decompress(&packed, Some(format), generous).unwrap().len(),
            zeros.len()
        );
    }
    let small = vec![0_u8; 512 * 1024];
    for format in ALL {
        let packed = compressed(format, &small);
        assert_eq!(decompress(&packed, Some(format), LIMITS).unwrap(), small);
    }
}

#[test]
fn levels_outside_the_format_range_are_refused() {
    for (format, level) in [
        (Format::Gzip, 10),
        (Format::Deflate, -1),
        (Format::DeflateRaw, 10),
        (Format::Brotli, 12),
    ] {
        assert_eq!(
            compress(b"x", format, Some(level)),
            Err(CodecError::InvalidLevel)
        );
    }
    assert!(compress(b"x", Format::Brotli, Some(11)).is_ok());
}

#[test]
fn ordinals_match_the_compress_format_enum() {
    assert_eq!(Format::from_ordinal(0), Some(Format::Gzip));
    assert_eq!(Format::from_ordinal(1), Some(Format::Deflate));
    assert_eq!(Format::from_ordinal(2), Some(Format::DeflateRaw));
    assert_eq!(Format::from_ordinal(3), Some(Format::Brotli));
    assert_eq!(Format::from_ordinal(4), None);
}

#[test]
fn adler32_matches_the_reference_value() {
    assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    assert_eq!(adler32(b""), 1);
    let long = vec![0xff_u8; 100_000];
    let mut reference = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    reference.write_all(&long).unwrap();
    let encoded = reference.finish().unwrap();
    let trailer = &encoded[encoded.len() - 4..];
    assert_eq!(
        adler32(&long),
        u32::from_be_bytes([trailer[0], trailer[1], trailer[2], trailer[3]])
    );
}
