//! Pure canonical codecs behind `tyrus:encoding`.
//!
//! This crate holds the rules of every codec and nothing else: no FFI, no C
//! allocator, no thread-local error slot, and no process abort. The runtime maps
//! a `None` or a [`RadixFailure`] to its own `EncodingError` codes. Keeping the
//! rules here, away from the ABI, is what lets them be verified formally: the
//! runtime crate cannot be compiled by the verifier, and a harness that modeled
//! its raw pointers would prove the model rather than the codec.
//!
//! Every codec has an allocation-free core that writes into a buffer the caller
//! provides (`*_into`), and a thin wrapper that allocates exactly once and
//! delegates to it. The proofs go through the cores. A symbolic `String` or
//! `Vec` makes the verifier model the allocator, its copies, and its growth,
//! and that model is what made the first harnesses exhaust the machine's memory
//! before the solver even started.
//!
//! Decoders take bytes rather than `&str`. A decoder only ever inspects ASCII,
//! and taking bytes keeps the proofs off `from_utf8` over symbolic input, which
//! is the search `kani.md` records as the one that does not converge.

#![forbid(unsafe_code)]

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const BASE64URL_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

pub const RADIX_MAX_LEN: usize = 65;

pub fn utf16_from_bytes(bytes: &[u8], little_endian: bool) -> Option<Vec<u16>> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    Some(
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if little_endian {
                    u16::from_le_bytes(*pair)
                } else {
                    u16::from_be_bytes(*pair)
                }
            })
            .collect(),
    )
}

pub fn decode_utf16(units: &[u16], lossy: bool) -> Option<String> {
    let decoded = std::char::decode_utf16(units.iter().copied());
    if lossy {
        Some(
            decoded
                .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect(),
        )
    } else {
        decoded.collect::<Result<String, _>>().ok()
    }
}

fn ascii_string(mut buffer: Vec<u8>, written: Option<usize>) -> String {
    buffer.truncate(written.expect("the wrapper sizes the buffer for the input"));
    String::from_utf8(buffer).expect("every encoder writes ASCII")
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

pub fn hex_encode_into(bytes: &[u8], out: &mut [u8]) -> Option<usize> {
    let written = bytes.len().checked_mul(2)?;
    let out = out.get_mut(..written)?;
    for (byte, pair) in bytes.iter().zip(out.as_chunks_mut::<2>().0) {
        pair[0] = HEX_DIGITS[(byte >> 4) as usize];
        pair[1] = HEX_DIGITS[(byte & 0x0f) as usize];
    }
    Some(written)
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut buffer = vec![0; bytes.len() * 2];
    let written = hex_encode_into(bytes, &mut buffer);
    ascii_string(buffer, written)
}

pub fn hex_decode_into(text: &[u8], out: &mut [u8]) -> Option<usize> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let count = text.len() / 2;
    let out = out.get_mut(..count)?;
    for (pair, byte) in text.as_chunks::<2>().0.iter().zip(out) {
        *byte = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Some(count)
}

pub fn hex_decode(text: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0; text.len() / 2];
    let count = hex_decode_into(text, &mut out)?;
    out.truncate(count);
    Some(out)
}

pub fn base64_encoded_len(len: usize, url_safe: bool) -> usize {
    let full = len / 3 * 4;
    match (len % 3, url_safe) {
        (0, _) => full,
        (_, false) => full + 4,
        (1, true) => full + 2,
        (_, true) => full + 3,
    }
}

pub fn base64_encode_into(bytes: &[u8], url_safe: bool, out: &mut [u8]) -> Option<usize> {
    let alphabet = if url_safe {
        BASE64URL_ALPHABET
    } else {
        BASE64_ALPHABET
    };
    let written = base64_encoded_len(bytes.len(), url_safe);
    let out = out.get_mut(..written)?;
    let mut position = 0;
    let mut emit = |symbol: u8| {
        out[position] = symbol;
        position += 1;
    };
    for chunk in bytes.chunks(3) {
        let value = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        emit(alphabet[((value >> 18) & 0x3f) as usize]);
        emit(alphabet[((value >> 12) & 0x3f) as usize]);
        if chunk.len() > 1 {
            emit(alphabet[((value >> 6) & 0x3f) as usize]);
        } else if !url_safe {
            emit(b'=');
        }
        if chunk.len() > 2 {
            emit(alphabet[(value & 0x3f) as usize]);
        } else if !url_safe {
            emit(b'=');
        }
    }
    Some(written)
}

pub fn base64_encode(bytes: &[u8], url_safe: bool) -> String {
    let mut buffer = vec![0; base64_encoded_len(bytes.len(), url_safe)];
    let written = base64_encode_into(bytes, url_safe, &mut buffer);
    ascii_string(buffer, written)
}

fn base64_value(byte: u8, url_safe: bool) -> Option<u32> {
    Some(match byte {
        b'A'..=b'Z' => (byte - b'A') as u32,
        b'a'..=b'z' => (byte - b'a' + 26) as u32,
        b'0'..=b'9' => (byte - b'0' + 52) as u32,
        b'+' if !url_safe => 62,
        b'/' if !url_safe => 63,
        b'-' if url_safe => 62,
        b'_' if url_safe => 63,
        _ => return None,
    })
}

pub fn base64_decode_into(bytes: &[u8], url_safe: bool, out: &mut [u8]) -> Option<usize> {
    let content: &[u8] = if url_safe {
        if bytes.len() % 4 == 1 {
            return None;
        }
        bytes
    } else {
        if !bytes.len().is_multiple_of(4) {
            return None;
        }
        let padding = match bytes {
            [.., b'=', b'='] => 2,
            [.., b'='] => 1,
            _ => 0,
        };
        let content = &bytes[..bytes.len() - padding];
        if padding == 1 && content.len() % 4 != 3 || padding == 2 && content.len() % 4 != 2 {
            return None;
        }
        content
    };
    let mut count = 0;
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for &byte in content {
        let value = base64_value(byte, url_safe)?;
        buffer = ((buffer << 6) | value) & 0x3fff;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            *out.get_mut(count)? = (buffer >> bits) as u8;
            count += 1;
        }
    }
    if bits > 0 && buffer & ((1 << bits) - 1) != 0 {
        return None;
    }
    Some(count)
}

pub fn base64_decode(bytes: &[u8], url_safe: bool) -> Option<Vec<u8>> {
    let mut out = vec![0; bytes.len() / 4 * 3 + 3];
    let count = base64_decode_into(bytes, url_safe, &mut out)?;
    out.truncate(count);
    Some(out)
}

pub fn base32_encoded_len(len: usize) -> usize {
    len.div_ceil(5) * 8
}

pub fn base32_encode_into(bytes: &[u8], out: &mut [u8]) -> Option<usize> {
    let written = base32_encoded_len(bytes.len());
    let out = out.get_mut(..written)?;
    for (chunk, group) in bytes.chunks(5).zip(out.as_chunks_mut::<8>().0) {
        let bits = chunk.len() * 8;
        let characters = bits.div_ceil(5);
        let field = characters * 5;
        let mut buffer: u64 = 0;
        for &byte in chunk {
            buffer = (buffer << 8) | byte as u64;
        }
        buffer <<= field - bits;
        for (index, symbol) in group.iter_mut().enumerate() {
            *symbol = if index < characters {
                BASE32_ALPHABET[((buffer >> (field - 5 * (index + 1))) & 0x1f) as usize]
            } else {
                b'='
            };
        }
    }
    Some(written)
}

pub fn base32_encode(bytes: &[u8]) -> String {
    let mut buffer = vec![0; base32_encoded_len(bytes.len())];
    let written = base32_encode_into(bytes, &mut buffer);
    ascii_string(buffer, written)
}

fn base32_value(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some((byte - b'A') as u32),
        b'2'..=b'7' => Some((byte - b'2' + 26) as u32),
        _ => None,
    }
}

pub fn base32_decode_into(bytes: &[u8], out: &mut [u8]) -> Option<usize> {
    if bytes.is_empty() {
        return Some(0);
    }
    if !bytes.len().is_multiple_of(8) {
        return None;
    }
    let padding = bytes.iter().rev().take_while(|&&byte| byte == b'=').count();
    let content = &bytes[..bytes.len() - padding];
    if content.is_empty() || !matches!(content.len() % 8, 0 | 2 | 4 | 5 | 7) {
        return None;
    }
    let mut count = 0;
    let mut buffer: u64 = 0;
    let mut bits: u32 = 0;
    for &byte in content {
        let value = base32_value(byte)?;
        buffer = ((buffer << 5) | value as u64) & 0x1fff;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            *out.get_mut(count)? = (buffer >> bits) as u8;
            count += 1;
        }
    }
    if bits > 0 && buffer & ((1 << bits) - 1) != 0 {
        return None;
    }
    Some(count)
}

pub fn base32_decode(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = vec![0; bytes.len() / 8 * 5 + 5];
    let count = base32_decode_into(bytes, &mut out)?;
    out.truncate(count);
    Some(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadixFailure {
    InvalidBase,
    InvalidInput,
    Overflow,
}

pub fn valid_radix_base(base: i32) -> bool {
    matches!(base, 2 | 8 | 10 | 16)
}

fn radix_digit(byte: u8, base: u32) -> Option<u32> {
    let value = match byte {
        b'0'..=b'9' => (byte - b'0') as u32,
        b'a'..=b'f' => (byte - b'a' + 10) as u32,
        _ => return None,
    };
    (value < base).then_some(value)
}

pub fn radix_format_i64_into(
    value: i64,
    base: i32,
    out: &mut [u8; RADIX_MAX_LEN],
) -> Result<usize, RadixFailure> {
    if !valid_radix_base(base) {
        return Err(RadixFailure::InvalidBase);
    }
    let base = base as u64;
    let mut magnitude = value.unsigned_abs();
    let mut start = RADIX_MAX_LEN;
    loop {
        start -= 1;
        out[start] = HEX_DIGITS[(magnitude % base) as usize];
        magnitude /= base;
        if magnitude == 0 {
            break;
        }
    }
    if value < 0 {
        start -= 1;
        out[start] = b'-';
    }
    Ok(start)
}

pub fn radix_format_i64(value: i64, base: i32) -> Result<String, RadixFailure> {
    let mut buffer = [0u8; RADIX_MAX_LEN];
    let start = radix_format_i64_into(value, base, &mut buffer)?;
    Ok(String::from_utf8(buffer[start..].to_vec()).expect("radix digits are ASCII"))
}

fn radix_magnitude(digits: &[u8], base: u32, limit: u64) -> Result<u64, RadixFailure> {
    if digits.is_empty() || !digits.iter().all(|&byte| radix_digit(byte, base).is_some()) {
        return Err(RadixFailure::InvalidInput);
    }
    digits.iter().try_fold(0u64, |magnitude, &byte| {
        let digit = radix_digit(byte, base).ok_or(RadixFailure::InvalidInput)?;
        magnitude
            .checked_mul(base as u64)
            .and_then(|value| value.checked_add(digit as u64))
            .filter(|&value| value <= limit)
            .ok_or(RadixFailure::Overflow)
    })
}

pub fn radix_parse_i64(text: &[u8], base: i32) -> Result<i64, RadixFailure> {
    if !valid_radix_base(base) {
        return Err(RadixFailure::InvalidBase);
    }
    let base = base as u32;
    match text.split_first() {
        Some((b'-', digits)) => {
            let magnitude = radix_magnitude(digits, base, i64::MIN.unsigned_abs())?;
            Ok(0i64.wrapping_sub_unsigned(magnitude))
        }
        Some((b'+', _)) => Err(RadixFailure::InvalidInput),
        _ => {
            let magnitude = radix_magnitude(text, base, i64::MAX as u64)?;
            Ok(magnitude as i64)
        }
    }
}

pub fn radix_parse_u64(text: &[u8], base: i32) -> Result<u64, RadixFailure> {
    if !valid_radix_base(base) {
        return Err(RadixFailure::InvalidBase);
    }
    radix_magnitude(text, base as u32, u64::MAX)
}

#[cfg(kani)]
#[path = "kani_proofs/kani_proofs.rs"]
mod kani_proofs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips_and_rejects_non_canonical_text() {
        let bytes = [0xde, 0xad, 0xbe, 0xef, 0x00, 0xff];
        assert_eq!(hex_encode(&bytes), "deadbeef00ff");
        assert_eq!(hex_decode(b"deadbeef00ff").unwrap(), bytes);
        for text in ["abc", "0x41", "41 42", "ZZ", "DEad", "4"] {
            assert_eq!(hex_decode(text.as_bytes()), None, "`{text}`");
        }
    }

    #[test]
    fn base64_variants_round_trip_rfc_4648_vectors() {
        for (bytes, standard, url) in [
            (&b""[..], "", ""),
            (b"f", "Zg==", "Zg"),
            (b"fo", "Zm8=", "Zm8"),
            (b"foo", "Zm9v", "Zm9v"),
            (b"foob", "Zm9vYg==", "Zm9vYg"),
            (&[0xfb, 0xff][..], "+/8=", "-_8"),
        ] {
            assert_eq!(base64_encode(bytes, false), standard);
            assert_eq!(base64_encode(bytes, true), url);
            assert_eq!(base64_decode(standard.as_bytes(), false).unwrap(), bytes);
            assert_eq!(base64_decode(url.as_bytes(), true).unwrap(), bytes);
        }
        for text in [
            "Zg",
            "Zg=",
            "Zg===",
            "Zg=c",
            "Z===",
            "=Zm9",
            "-_-_",
            "Z g=",
            "AAEC3q2+7z==",
        ] {
            assert_eq!(base64_decode(text.as_bytes(), false), None, "`{text}`");
        }
        for text in ["Zg==", "+/8", "1", "Zh"] {
            assert_eq!(base64_decode(text.as_bytes(), true), None, "`{text}`");
        }
    }

    #[test]
    fn base32_round_trips_rfc_4648_vectors() {
        for (bytes, encoded) in [
            (&b""[..], ""),
            (b"f", "MY======"),
            (b"fo", "MZXQ===="),
            (b"foo", "MZXW6==="),
            (b"foob", "MZXW6YQ="),
            (b"fooba", "MZXW6YTB"),
            (b"foobar", "MZXW6YTBOI======"),
        ] {
            assert_eq!(base32_encode(bytes), encoded);
            assert_eq!(base32_decode(encoded.as_bytes()).unwrap(), bytes);
        }
        for text in [
            "my======",
            "MZXW6YQ",
            "========",
            "MZXW7===",
            "MZ======",
            "MZXW6YQ=Y=",
        ] {
            assert_eq!(base32_decode(text.as_bytes()), None, "`{text}`");
        }
    }

    #[test]
    fn allocation_free_cores_refuse_a_short_buffer_instead_of_truncating() {
        let mut two = [0u8; 2];
        assert_eq!(hex_encode_into(&[0xab, 0xcd], &mut two), None);
        assert_eq!(base64_encode_into(b"foo", false, &mut two), None);
        assert_eq!(base32_encode_into(b"f", &mut two), None);
        let mut one = [0u8; 1];
        assert_eq!(hex_decode_into(b"abcd", &mut one), None);
        assert_eq!(base64_decode_into(b"Zm8=", false, &mut one), None);
        assert_eq!(base32_decode_into(b"MZXQ====", &mut one), None);
    }

    #[test]
    fn utf16_rejects_odd_bytes_and_unpaired_surrogates() {
        assert_eq!(utf16_from_bytes(&[0x41], true), None);
        assert_eq!(
            utf16_from_bytes(&[0x41, 0x00, 0x3d, 0xd8], true).unwrap(),
            [0x0041, 0xd83d]
        );
        assert_eq!(decode_utf16(&[0xd800], false), None);
        assert_eq!(decode_utf16(&[0xd800], true).unwrap(), "\u{fffd}");
        assert_eq!(
            decode_utf16(&[0x0041, 0xd83d, 0xde00], false).unwrap(),
            "A😀"
        );
    }

    #[test]
    fn radix_formats_and_parses_every_base_at_the_limits() {
        for (value, base, text) in [
            (0, 2, "0"),
            (255, 2, "11111111"),
            (255, 8, "377"),
            (-255, 16, "-ff"),
            (i64::MIN, 10, "-9223372036854775808"),
            (
                i64::MIN,
                2,
                "-1000000000000000000000000000000000000000000000000000000000000000",
            ),
            (i64::MAX, 16, "7fffffffffffffff"),
        ] {
            assert_eq!(radix_format_i64(value, base).unwrap(), text);
            assert_eq!(radix_parse_i64(text.as_bytes(), base), Ok(value));
        }
        assert_eq!(radix_parse_u64(b"ffffffffffffffff", 16), Ok(u64::MAX));
        assert_eq!(radix_format_i64(10, 3), Err(RadixFailure::InvalidBase));
    }

    #[test]
    fn radix_parse_separates_invalid_input_from_overflow() {
        use RadixFailure::*;
        for (text, base, failure) in [
            ("", 10, InvalidInput),
            ("-", 10, InvalidInput),
            ("+10", 10, InvalidInput),
            ("FF", 16, InvalidInput),
            ("0x10", 16, InvalidInput),
            ("12", 2, InvalidInput),
            ("9", 8, InvalidInput),
            ("10", 3, InvalidBase),
            ("9223372036854775808", 10, Overflow),
            ("-9223372036854775809", 10, Overflow),
            ("99999999999999999999z", 10, InvalidInput),
        ] {
            assert_eq!(
                radix_parse_i64(text.as_bytes(), base),
                Err(failure),
                "`{text}`"
            );
        }
        assert_eq!(radix_parse_u64(b"-1", 10), Err(InvalidInput));
        assert_eq!(radix_parse_u64(b"18446744073709551616", 10), Err(Overflow));
    }
}
