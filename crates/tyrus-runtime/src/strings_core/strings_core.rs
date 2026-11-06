//! What the string operations compute, separated from how they cross the ABI.
//!
//! Every function here takes `&str` and returns a `String`, an index or a
//! boolean. None of them knows what a `*const c_char` is, and none allocates a
//! representation, the entry points in [`crate::strings`] do that.
//!
//! # Why the split exists
//!
//! The representation is moving from a NUL-terminated pointer to a pointer plus
//! a length (see `../tyrus-roadmaps/string-representation.md`), and that change
//! is atomic across the whole FFI surface. Every one of these operations had its
//! logic braided together with three ABI concerns: reading a C string in,
//! deciding what a null pointer means, and allocating a C string out. Migrating
//! them in that state means rewriting the logic and the boundary in the same
//! edit, on two hundred functions, with the tree red throughout.
//!
//! Split, the boundary is the only thing left to change. `slice` computes a
//! substring; whether that substring is handed back as a `CString` or as a
//! sixteen-byte value is a question the caller answers. The flip rewrites the
//! answer once per function instead of rewriting the function.
//!
//! # Indices are characters
//!
//! Every index and length in this module counts Unicode scalar values, never
//! UTF-8 bytes, so that positions agree with `length` and `chars()`. The
//! conversion to byte offsets happens here and stays here.

use tyrus_validators::{
    byte_offset_of_char, char_count as char_len, char_index_of_byte as char_index,
};

fn byte_offset(text: &str, chars: usize, index: usize) -> usize {
    if text.len() == chars {
        return index.min(text.len());
    }
    byte_offset_of_char(text, index)
}

pub(crate) const INDEX_UNSET: i64 = i64::MAX;

fn normalize_index(index: i64, length: i64) -> i64 {
    if index < 0 {
        (length + index).max(0)
    } else {
        index.min(length)
    }
}

fn clamp_byte_index(index: i64, len: usize) -> usize {
    if index < 0 {
        0
    } else {
        (index as usize).min(len)
    }
}

pub(crate) fn char_at(text: &str, chars: usize, index: i64) -> String {
    let index = normalize_index(index, chars as i64) as usize;
    text.chars()
        .nth(index)
        .map(String::from)
        .unwrap_or_default()
}

pub(crate) fn char_code_at(text: &str, chars: usize, index: i64) -> i32 {
    let index = normalize_index(index, chars as i64) as usize;
    text.chars()
        .nth(index)
        .map_or(-1, |character| character as i32)
}

pub(crate) fn count_occurrences(text: &str, search: &str) -> i64 {
    if search.is_empty() {
        return 0;
    }
    text.matches(search).count() as i64
}

pub(crate) fn index_of(text: &str, chars: usize, search: &str, from_index: i64) -> i64 {
    let len = chars;
    if search.is_empty() {
        return clamp_byte_index(from_index, len) as i64;
    }
    let from = from_index.max(0) as usize;
    if from >= len {
        return -1;
    }
    let start = byte_offset(text, chars, from);
    text[start..]
        .find(search)
        .map_or(-1, |offset| char_index(text, start + offset) as i64)
}

pub(crate) fn last_index_of(text: &str, chars: usize, search: &str, from_index: i64) -> i64 {
    let len = chars;
    if search.is_empty() {
        return clamp_byte_index(from_index, len) as i64;
    }
    let search_len = char_len(search);
    let window_end = if from_index == INDEX_UNSET {
        len
    } else if from_index < 0 {
        search_len.min(len)
    } else if from_index as usize >= len {
        len
    } else {
        (from_index as usize)
            .min(len.saturating_sub(search_len))
            .saturating_add(search_len)
    };
    if window_end < search_len {
        return -1;
    }
    let end = byte_offset(text, chars, window_end);
    text[..end]
        .rfind(search)
        .map_or(-1, |offset| char_index(text, offset) as i64)
}

pub(crate) fn slice(text: &str, chars: usize, start: i64, end: i64) -> &str {
    let len = chars as i64;
    let start = normalize_index(start, len);
    let end = if end == INDEX_UNSET {
        len
    } else {
        normalize_index(end, len)
    };
    if start >= end {
        return "";
    }
    &text[byte_offset(text, chars, start as usize)..byte_offset(text, chars, end as usize)]
}

pub(crate) fn substring(text: &str, chars: usize, start: i64, end: i64) -> &str {
    let len = chars as i64;
    let start = start.clamp(0, len);
    let end = if end == INDEX_UNSET {
        len
    } else {
        end.clamp(0, len)
    };
    let (start, end) = if start > end {
        (end, start)
    } else {
        (start, end)
    };
    if start >= end {
        return "";
    }
    &text[byte_offset(text, chars, start as usize)..byte_offset(text, chars, end as usize)]
}

pub(crate) fn replace_first(text: &str, search: &str, replacement: &str) -> String {
    text.replacen(search, replacement, 1)
}

pub(crate) fn repeat(text: &str, count: i64) -> String {
    let count = count.max(0) as usize;
    if count == 0 || text.is_empty() {
        return String::new();
    }
    text.repeat(count)
}

fn padding(chars: usize, target_length: i64, pad: &str) -> Option<String> {
    let target = target_length.max(0) as usize;
    if chars >= target || pad.is_empty() {
        return None;
    }
    Some(pad.chars().cycle().take(target - chars).collect())
}

pub(crate) fn pad_start(text: &str, chars: usize, target_length: i64, pad: &str) -> String {
    match padding(chars, target_length, pad) {
        Some(padding) => format!("{padding}{text}"),
        None => text.to_string(),
    }
}

pub(crate) fn pad_end(text: &str, chars: usize, target_length: i64, pad: &str) -> String {
    match padding(chars, target_length, pad) {
        Some(padding) => format!("{text}{padding}"),
        None => text.to_string(),
    }
}

pub(crate) fn concat(text: &str, other: &str) -> String {
    let mut result = String::with_capacity(text.len() + other.len());
    result.push_str(text);
    result.push_str(other);
    result
}

pub(crate) fn split<'a>(text: &'a str, separator: &str) -> Vec<&'a str> {
    if separator.is_empty() {
        text.split("").filter(|part| !part.is_empty()).collect()
    } else {
        text.split(separator).collect()
    }
}

fn tokenize_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut prev_was_upper = false;
    let mut prev_was_separator = true;

    for c in text.chars() {
        if c == ' ' || c == '_' || c == '-' {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            prev_was_upper = false;
            prev_was_separator = true;
        } else if c.is_uppercase() {
            if !current.is_empty() && !prev_was_upper && !prev_was_separator {
                words.push(std::mem::take(&mut current));
            }
            current.push(c.to_lowercase().next().unwrap_or(c));
            prev_was_upper = true;
            prev_was_separator = false;
        } else {
            current.push(c);
            prev_was_upper = false;
            prev_was_separator = false;
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

pub(crate) fn to_slug_case(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    let normalized: String = text
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-')
        .collect::<String>();

    let words = tokenize_words(&normalized);
    words.join("-")
}

fn is_combining_mark(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x0300..=0x036F |
        0x1AB0..=0x1AFF |
        0x1DC0..=0x1DFF |
        0x20D0..=0x20FF |
        0xFE20..=0xFE2F
    )
}

pub(crate) fn to_dash_case(text: &str) -> String {
    let words = tokenize_words(text);
    words.join("-")
}

pub(crate) fn to_pascal_case(text: &str) -> String {
    let words = tokenize_words(text);
    let mut result = String::with_capacity(text.len());
    for word in &words {
        if let Some(first) = word.chars().next() {
            for c in first.to_uppercase() {
                result.push(c);
            }
            result.push_str(&word[first.len_utf8()..]);
        }
    }
    result
}

pub(crate) fn to_camel_case(text: &str) -> String {
    let words = tokenize_words(text);
    let mut result = String::with_capacity(text.len());
    for (i, word) in words.iter().enumerate() {
        if i == 0 {
            result.push_str(word);
        } else if let Some(first) = word.chars().next() {
            for c in first.to_uppercase() {
                result.push(c);
            }
            result.push_str(&word[first.len_utf8()..]);
        }
    }
    result
}

pub(crate) fn to_snake_case(text: &str) -> String {
    let words = tokenize_words(text);
    words.join("_")
}

pub(crate) fn truncate(text: &str, max_length: i64) -> String {
    let max = max_length.max(0) as usize;
    text.chars().take(max).collect()
}

pub(crate) fn truncate_ellipsis(text: &str, max_length: i64) -> String {
    let max = max_length.max(0) as usize;
    let char_count = text.chars().count();
    if char_count <= max {
        return text.to_string();
    }
    if max <= 3 {
        return text.chars().take(max).collect();
    }
    let mut result: String = text.chars().take(max - 3).collect();
    result.push_str("...");
    result
}

pub(crate) fn strip_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;

    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            '&' if !in_tag => {
                result.push(c);
            }
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    decode_html_entities(&result)
}

fn decode_html_entities(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            let mut entity = String::new();
            entity.push(c);
            while let Some(&next) = chars.peek() {
                if next == ';' {
                    entity.push(next);
                    chars.next();
                    break;
                }
                entity.push(next);
                chars.next();
                if entity.len() > 10 {
                    result.push_str(&entity);
                    entity.clear();
                    break;
                }
            }
            match entity.as_str() {
                "&lt;" => result.push('<'),
                "&gt;" => result.push('>'),
                "&amp;" => result.push('&'),
                "&quot;" => result.push('"'),
                "&#39;" | "&apos;" => result.push('\''),
                "&#60;" => result.push('<'),
                "&#62;" => result.push('>'),
                "&#38;" => result.push('&'),
                "&#34;" => result.push('"'),
                _ => result.push_str(&entity),
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub(crate) fn escape_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + text.len() / 4);
    for c in text.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

pub(crate) fn escape_debug(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + text.len() / 8);
    for character in text.chars() {
        match character {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            character if character.is_control() => {
                use core::fmt::Write;
                write!(result, "\\u{{{:x}}}", character as u32)
                    .expect("writing to String cannot fail");
            }
            character => result.push(character),
        }
    }
    result
}

pub(crate) fn url_encode(text: &str) -> String {
    use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};

    const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');

    percent_encode(text.as_bytes(), ENCODE_SET).collect::<String>()
}

pub(crate) fn url_decode(text: &str) -> String {
    match percent_encoding::percent_decode_str(text).decode_utf8() {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => text.to_string(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/strings_core.rs"]
mod tests;
