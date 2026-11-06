use crate::{
    byte_offset_of_char, char_byte_offset, char_count, char_count_bytes, char_index_at_byte,
    char_index_of_byte, normalize_char_index,
};

const MAX_CHARS: usize = 3;

fn symbolic_char() -> char {
    let width: u8 = kani::any();
    kani::assume(width < 4);
    match width {
        0 => 'a',
        1 => 'é',
        2 => '€',
        _ => '🚀',
    }
}

fn symbolic_text() -> String {
    let count: usize = kani::any();
    kani::assume(count <= MAX_CHARS);
    let mut text = String::new();
    for _ in 0..count {
        text.push(symbolic_char());
    }
    text
}

#[kani::proof]
#[kani::unwind(5)]
fn a_character_index_always_lands_on_a_character_boundary() {
    let text = symbolic_text();
    let index: usize = kani::any();
    kani::assume(index <= MAX_CHARS);

    let offset = char_byte_offset(&text, index);

    assert!(offset <= text.len());
    assert!(text.is_char_boundary(offset));
}

#[kani::proof]
#[kani::unwind(5)]
fn an_index_past_the_last_character_lands_on_the_end() {
    let text = symbolic_text();
    let index: usize = kani::any();
    kani::assume(index >= char_count(&text) && index <= MAX_CHARS);

    assert_eq!(char_byte_offset(&text, index), text.len());
}

#[kani::proof]
#[kani::unwind(5)]
fn a_character_position_survives_the_round_trip_through_bytes() {
    let text = symbolic_text();
    let index: usize = kani::any();
    kani::assume(index < char_count(&text));

    let offset = char_byte_offset(&text, index);

    assert_eq!(char_index_at_byte(&text, offset), index as i64);
}

#[kani::proof]
#[kani::unwind(5)]
fn a_normalized_index_never_leaves_the_character_range() {
    let text = symbolic_text();
    let count = char_count(&text);
    let index: i64 = kani::any();

    let normalized = normalize_char_index(index, count);

    assert!(normalized <= count);
    assert!(text.is_char_boundary(char_byte_offset(&text, normalized)));
}

#[kani::proof]
#[kani::unwind(5)]
fn a_negative_index_counts_back_from_the_end() {
    let count: usize = kani::any();
    kani::assume(count <= MAX_CHARS);
    let back: i64 = kani::any();
    kani::assume(back > 0 && back <= count as i64);

    assert_eq!(normalize_char_index(-back, count), count - back as usize);
}

const MAX_BYTES: usize = 4;

const MAX_COUNTED_BYTES: usize = 9;

fn any_utf8(storage: &[u8]) -> Option<&str> {
    core::str::from_utf8(storage).ok()
}

#[kani::proof]
#[kani::unwind(5)]
fn byte_offset_of_char_lands_on_a_character_boundary() {
    let storage: [u8; MAX_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTES);
    let Some(value) = any_utf8(&storage[..len]) else {
        return;
    };

    let index: usize = kani::any();

    let offset = byte_offset_of_char(value, index);

    assert!(offset <= value.len());
    assert!(value.is_char_boundary(offset));
}

#[kani::proof]
#[kani::unwind(5)]
fn an_out_of_range_index_saturates_at_the_end() {
    let storage: [u8; MAX_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTES);
    let Some(value) = any_utf8(&storage[..len]) else {
        return;
    };

    let index: usize = kani::any();
    kani::assume(index >= char_count(value));

    assert_eq!(byte_offset_of_char(value, index), value.len());
}

#[kani::proof]
#[kani::unwind(5)]
fn a_position_survives_the_round_trip_through_bytes() {
    let storage: [u8; MAX_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTES);
    let Some(value) = any_utf8(&storage[..len]) else {
        return;
    };

    let index: usize = kani::any();
    kani::assume(index < value.chars().count());

    let offset = byte_offset_of_char(value, index);

    assert_eq!(char_index_of_byte(value, offset), index);
}

#[kani::proof]
#[kani::unwind(14)]
fn the_word_at_a_time_counter_matches_the_reference() {
    let characters: [char; 3] = kani::any();

    let mut storage = [0u8; 12];
    let mut written = 0usize;
    for character in characters {
        let mut encoded = [0u8; 4];
        let width = character.encode_utf8(&mut encoded).len();
        storage[written..written + width].copy_from_slice(&encoded[..width]);
        written += width;
    }
    let bytes = &storage[..written];

    assert_eq!(char_count_bytes(bytes), 3);
    assert_eq!(
        char_count_bytes(bytes),
        bytes.iter().filter(|&&b| (b as i8) >= -0x40).count()
    );
}

#[kani::proof]
#[kani::unwind(11)]
fn the_counter_is_total_on_arbitrary_bytes() {
    let storage: [u8; MAX_COUNTED_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_COUNTED_BYTES);

    let counted = char_count_bytes(&storage[..len]);

    assert!(counted <= len);
}

#[kani::proof]
#[kani::unwind(5)]
fn char_count_never_exceeds_the_byte_length() {
    let storage: [u8; MAX_BYTES] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= MAX_BYTES);
    let Some(value) = any_utf8(&storage[..len]) else {
        return;
    };

    let characters = char_count(value);

    assert!(characters <= value.len());
    assert_eq!(characters == value.len(), value.is_ascii());
}
