use crate::*;

fn bounded<const N: usize>(storage: &[u8; N]) -> &[u8] {
    let len: usize = kani::any();
    kani::assume(len <= N);
    &storage[..len]
}

#[kani::proof]
#[kani::unwind(5)]
fn hex_decode_inverts_hex_encode() {
    let storage: [u8; 2] = kani::any();
    let bytes = bounded(&storage);
    let mut text = [0u8; 4];
    let Some(written) = hex_encode_into(bytes, &mut text) else {
        panic!("four characters hold two bytes");
    };
    let mut decoded = [0u8; 2];
    assert!(hex_decode_into(&text[..written], &mut decoded) == Some(bytes.len()));
    assert!(decoded[..bytes.len()] == *bytes);
}

#[kani::proof]
#[kani::unwind(5)]
fn hex_decode_accepts_only_what_hex_encode_produces() {
    let storage: [u8; 4] = kani::any();
    let text = bounded(&storage);
    let mut decoded = [0u8; 2];
    if let Some(count) = hex_decode_into(text, &mut decoded) {
        let mut again = [0u8; 4];
        let Some(written) = hex_encode_into(&decoded[..count], &mut again) else {
            panic!("four characters hold two bytes");
        };
        assert!(again[..written] == *text);
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn base64_decode_inverts_base64_encode() {
    let url_safe: bool = kani::any();
    let storage: [u8; 4] = kani::any();
    let bytes = bounded(&storage);
    let mut text = [0u8; 8];
    let Some(written) = base64_encode_into(bytes, url_safe, &mut text) else {
        panic!("eight characters hold four bytes");
    };
    let mut decoded = [0u8; 4];
    assert!(base64_decode_into(&text[..written], url_safe, &mut decoded) == Some(bytes.len()));
    assert!(decoded[..bytes.len()] == *bytes);
}

#[kani::proof]
#[kani::unwind(9)]
fn base64_decode_accepts_only_what_base64_encode_produces() {
    let url_safe: bool = kani::any();
    let storage: [u8; 8] = kani::any();
    let text = bounded(&storage);
    let mut decoded = [0u8; 6];
    if let Some(count) = base64_decode_into(text, url_safe, &mut decoded) {
        let mut again = [0u8; 8];
        let Some(written) = base64_encode_into(&decoded[..count], url_safe, &mut again) else {
            panic!(
                "eight characters of text decode to at most six bytes, which re-encode to eight"
            );
        };
        assert!(again[..written] == *text);
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn base32_decode_inverts_base32_encode() {
    let storage: [u8; 5] = kani::any();
    let bytes = bounded(&storage);
    let mut text = [0u8; 8];
    let Some(written) = base32_encode_into(bytes, &mut text) else {
        panic!("eight characters hold five bytes");
    };
    let mut decoded = [0u8; 5];
    assert!(base32_decode_into(&text[..written], &mut decoded) == Some(bytes.len()));
    assert!(decoded[..bytes.len()] == *bytes);
}

#[kani::proof]
#[kani::unwind(9)]
fn base32_decode_accepts_only_what_base32_encode_produces() {
    let storage: [u8; 8] = kani::any();
    let text = bounded(&storage);
    let mut decoded = [0u8; 5];
    if let Some(count) = base32_decode_into(text, &mut decoded) {
        let mut again = [0u8; 8];
        let Some(written) = base32_encode_into(&decoded[..count], &mut again) else {
            panic!("eight characters hold five bytes");
        };
        assert!(again[..written] == *text);
    }
}

fn radix_round_trip(base: i32) {
    let value: i64 = kani::any();
    let mut buffer = [0u8; RADIX_MAX_LEN];
    let Ok(start) = radix_format_i64_into(value, base, &mut buffer) else {
        panic!("the base is supported");
    };
    assert!(radix_parse_i64(&buffer[start..], base) == Ok(value));
}

#[kani::proof]
#[kani::unwind(67)]
fn radix_parse_inverts_format_in_base_2() {
    radix_round_trip(2);
}

#[kani::proof]
#[kani::unwind(25)]
fn radix_parse_inverts_format_in_base_8() {
    radix_round_trip(8);
}

#[kani::proof]
#[kani::unwind(22)]
fn radix_parse_inverts_format_in_base_10() {
    radix_round_trip(10);
}

#[kani::proof]
#[kani::unwind(19)]
fn radix_parse_inverts_format_in_base_16() {
    radix_round_trip(16);
}

#[kani::proof]
#[kani::unwind(6)]
fn radix_parse_is_total_on_arbitrary_text() {
    let storage: [u8; 4] = kani::any();
    let text = bounded(&storage);
    let base: i32 = kani::any();
    let signed = radix_parse_i64(text, base);
    let unsigned = radix_parse_u64(text, base);
    if !valid_radix_base(base) {
        assert!(signed == Err(RadixFailure::InvalidBase));
        assert!(unsigned == Err(RadixFailure::InvalidBase));
    }
}
