//! Native string operations exposed across the ABI.
//!
//! Every function here takes and returns [`TyrusString`], a pointer plus both
//! lengths, sixteen bytes travelling in two registers. What each operation
//! computes lives in [`crate::strings_core`]; this module is only the boundary.
//!
//! The old shape was `*const c_char` in and a heap `*mut c_char` out, which
//! meant every call opened with a `strlen` to find where its input ended. That
//! scan is gone: the length arrives with the pointer.

use super::*;
extern crate core as rust_core;
use rust_core::fmt::NumBuffer;

use crate::strings_core as core;
use tyrus_string::TyrusString;

/// Borrows the text of a string, or fails when there is none.
///
/// This is where the `strlen` used to be. The value already carries its byte
/// length, so the borrow is two field reads and a slice, no scan, and no
/// validation pass either, since the constructors only accept UTF-8.
///
/// # Safety
///
/// The returned lifetime is chosen by the caller and is not tied to anything
/// real, so it must not outlive the buffer. Every caller is an `extern "C"`
/// entry point that uses the borrow within the call and never stores it, which
/// the module contract already requires: the buffer stays valid for the
/// complete call.
fn text<'a>(value: &TyrusString) -> Result<&'a str, ()> {
    if value.is_null() {
        return Err(());
    }
    // SAFETY: the buffer outlives the call, per the module contract above.
    Ok(unsafe { std::mem::transmute::<&str, &'a str>(value.as_str()) })
}

pub(crate) fn alloc_string(value: &str) -> TyrusString {
    TyrusString::from_str_owned(value).unwrap_or_else(TyrusString::null)
}

fn panic_message(message: String) -> ! {
    let message = CString::new(message)
        .unwrap_or_else(|_| CString::new("tyrus runtime error").expect("no NUL"));
    unsafe { tyrus_panic(message.as_ptr()) }
}

#[cfg(test)]
use crate::strings_core::INDEX_UNSET;

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_f64(value: f64) -> TyrusString {
    let formatted = if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value.is_finite() && value == value.trunc() {
        format!("{value:.1}")
    } else {
        format!("{value}")
    };
    alloc_string(&formatted)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_f32(value: f32) -> TyrusString {
    let formatted = if value.is_nan() {
        "NaN".to_string()
    } else if value == f32::INFINITY {
        "Infinity".to_string()
    } else if value == f32::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value.is_finite() && value == value.trunc() {
        format!("{value:.1}")
    } else {
        format!("{value}")
    };
    alloc_string(&formatted)
}

/// Format an `f128` from its two raw halves. The value cannot cross the ABI as
/// a number (Rust has no stable `f128`) so the generated code hands over the
/// bit pattern and [`crate::binary128`] produces the shortest text that still
/// round-trips, the same guarantee `f32`/`f64` printing gives.
/// Returns a heap C string owned by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_f128(low: u64, high: u64) -> TyrusString {
    alloc_string(&crate::binary128::format(low, high))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_i128(value: i128) -> TyrusString {
    let mut buffer = NumBuffer::<i128>::new();
    alloc_string(value.format_into(&mut buffer))
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_format_i128_indirect(value: *const i128) -> TyrusString {
    let Some(value) = (unsafe { value.as_ref() }) else {
        return TyrusString::null();
    };
    tyrus_format_i128(*value)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_f64_precision(value: f64, precision: u32) -> TyrusString {
    alloc_string(&format!("{value:.*}", precision as usize))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_i128_padded(value: i128, width: u32) -> TyrusString {
    alloc_string(&format!("{value:0width$}", width = width as usize))
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_format_i128_padded_indirect(
    value: *const i128,
    width: u32,
) -> TyrusString {
    let Some(value) = (unsafe { value.as_ref() }) else {
        return TyrusString::null();
    };
    tyrus_format_i128_padded(*value, width)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_u128_padded(value: u128, width: u32) -> TyrusString {
    alloc_string(&format!("{value:0width$}", width = width as usize))
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_format_u128_padded_indirect(
    value: *const u128,
    width: u32,
) -> TyrusString {
    let Some(value) = (unsafe { value.as_ref() }) else {
        return TyrusString::null();
    };
    tyrus_format_u128_padded(*value, width)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_char(value: u32) -> TyrusString {
    let character = char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER);
    alloc_string(&character.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_is_valid(value: u32) -> bool {
    char::from_u32(value).is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_format_u128(value: u128) -> TyrusString {
    let mut buffer = NumBuffer::<u128>::new();
    alloc_string(value.format_into(&mut buffer))
}

#[cfg(windows)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_format_u128_indirect(value: *const u128) -> TyrusString {
    let Some(value) = (unsafe { value.as_ref() }) else {
        return TyrusString::null();
    };
    tyrus_format_u128(*value)
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

unsafe fn box_value<T: Copy>(value: T) -> *mut T {
    unsafe {
        let storage = malloc(std::mem::size_of::<T>().max(1)).cast::<T>();
        if !storage.is_null() {
            *storage = value;
        }
        storage
    }
}

fn leading_number(text: &str, allow_fraction: bool) -> Option<&str> {
    let text = text.trim_start();
    let bytes = text.as_bytes();
    let mut end = 0;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    let digits_start = end;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if allow_fraction && end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if allow_fraction
        && end < bytes.len()
        && (bytes[end] == b'e' || bytes[end] == b'E')
        && end > digits_start
    {
        let mut exp = end + 1;
        if exp < bytes.len() && (bytes[exp] == b'+' || bytes[exp] == b'-') {
            exp += 1;
        }
        if exp < bytes.len() && bytes[exp].is_ascii_digit() {
            end = exp;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
        }
    }
    if text[digits_start..end].bytes().any(|b| b.is_ascii_digit()) {
        Some(&text[..end])
    } else {
        None
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_i64(text: TyrusString) -> *mut i64 {
    unsafe {
        let Ok(text) = self::text(&text) else {
            return std::ptr::null_mut();
        };
        match leading_number(text, false).and_then(|prefix| prefix.parse::<i64>().ok()) {
            Some(value) => box_value(value),
            None => std::ptr::null_mut(),
        }
    }
}

macro_rules! parse_int_at_width {
    ($name:ident, $ty:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(text: TyrusString) -> *mut $ty {
            unsafe {
                let Ok(text) = self::text(&text) else {
                    return std::ptr::null_mut();
                };
                match leading_number(&text, false).and_then(|prefix| prefix.parse::<$ty>().ok()) {
                    Some(value) => box_value(value),
                    None => std::ptr::null_mut(),
                }
            }
        }
    };
}

parse_int_at_width!(tyrus_parse_i8, i8);
parse_int_at_width!(tyrus_parse_i16, i16);
parse_int_at_width!(tyrus_parse_i32, i32);
parse_int_at_width!(tyrus_parse_i128, i128);
parse_int_at_width!(tyrus_parse_isize, isize);
parse_int_at_width!(tyrus_parse_u8, u8);
parse_int_at_width!(tyrus_parse_u16, u16);
parse_int_at_width!(tyrus_parse_u32, u32);
parse_int_at_width!(tyrus_parse_u64, u64);
parse_int_at_width!(tyrus_parse_u128, u128);
parse_int_at_width!(tyrus_parse_usize, usize);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_f32_leading(text: TyrusString) -> *mut f32 {
    unsafe {
        let Ok(text) = self::text(&text) else {
            return std::ptr::null_mut();
        };
        match leading_number(text, true).and_then(|prefix| prefix.parse::<f32>().ok()) {
            Some(value) => box_value(value),
            None => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_f32_whole(text: TyrusString) -> *mut f32 {
    unsafe {
        let Ok(text) = self::text(&text) else {
            return std::ptr::null_mut();
        };
        match text.trim().parse::<f32>() {
            Ok(value) => box_value(value),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_f64_leading(text: TyrusString) -> *mut f64 {
    unsafe {
        let Ok(text) = self::text(&text) else {
            return std::ptr::null_mut();
        };
        match leading_number(text, true).and_then(|prefix| prefix.parse::<f64>().ok()) {
            Some(value) => box_value(value),
            None => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_parse_f64_whole(text: TyrusString) -> *mut f64 {
    unsafe {
        let Ok(text) = self::text(&text) else {
            return std::ptr::null_mut();
        };
        match text.trim().parse::<f64>() {
            Ok(value) => box_value(value),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_char_at(s: TyrusString, index: i64) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::char_at(text, s.char_len(), index))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_char_code_at(s: TyrusString, index: i64) -> i32 {
    let Ok(text) = text(&s) else {
        return -1;
    };
    core::char_code_at(text, s.char_len(), index)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_includes(s: TyrusString, search: TyrusString) -> i64 {
    let trace = cfg!(windows) && std::env::var_os("TYRUS_JIT_TRACE").is_some();
    if trace {
        eprintln!("JIT trace: string.includes entered");
    }
    let (Ok(text), Ok(search)) = (text(&s), text(&search)) else {
        if trace {
            eprintln!("JIT trace: string.includes received an invalid string");
        }
        return 0;
    };
    let found = text.contains(search);
    if trace {
        eprintln!("JIT trace: string.includes returned {found}");
    }
    i64::from(found)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_starts_with(s: TyrusString, search: TyrusString) -> i64 {
    let (Ok(text), Ok(search)) = (text(&s), text(&search)) else {
        return 0;
    };
    i64::from(text.starts_with(search))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_ends_with(s: TyrusString, search: TyrusString) -> i64 {
    let (Ok(text), Ok(search)) = (text(&s), text(&search)) else {
        return 0;
    };
    i64::from(text.ends_with(&search))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_count_occurrences(s: TyrusString, sub: TyrusString) -> i64 {
    let (Ok(text), Ok(sub)) = (text(&s), text(&sub)) else {
        return 0;
    };
    core::count_occurrences(text, sub)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_search_regex(s: TyrusString, pattern: TyrusString) -> i32 {
    let (Ok(text), Ok(pattern)) = (text(&s), text(&pattern)) else {
        return -1;
    };
    let Ok(regex) = regex::Regex::new(pattern) else {
        return -1;
    };
    regex.find(text).map_or(-1, |m| m.start() as i32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_matches_regex(s: TyrusString, pattern: TyrusString) -> i64 {
    let (Ok(text), Ok(pattern)) = (text(&s), text(&pattern)) else {
        return 0;
    };
    let Ok(regex) = regex::Regex::new(pattern) else {
        return 0;
    };
    i64::from(regex.is_match(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_exec(
    pattern: TyrusString,
    input: TyrusString,
    out: *mut TyrusArray,
) -> i64 {
    unsafe {
        let (Ok(pattern), Ok(input)) = (text(&pattern), text(&input)) else {
            return 0;
        };
        let Ok(regex) = regex::Regex::new(pattern) else {
            return 0;
        };
        let Some(captures) = regex.captures(input) else {
            return 0;
        };
        let Some(out) = out.as_mut() else {
            return 0;
        };
        let count = captures.len();
        let element_size = std::mem::size_of::<TyrusString>();
        let bytes = count.checked_mul(element_size).unwrap_or(0);
        let data = if bytes > 0 {
            let ptr = alloc_raw(bytes);
            if ptr.is_null() {
                return 0;
            }
            ptr
        } else {
            std::ptr::null_mut()
        };
        let ptrs: Vec<TyrusString> = captures
            .iter()
            .map(|m| {
                m.map_or(TyrusString::literal(""), |m| {
                    crate::strings::alloc_string(m.as_str())
                })
            })
            .collect();
        if count > 0 {
            std::ptr::copy_nonoverlapping(ptrs.as_ptr(), data.cast(), count);
        }
        *out = TyrusArray {
            data,
            length: count,
            capacity: count,
        };
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_exec_all(
    pattern: TyrusString,
    input: TyrusString,
    out: *mut TyrusArray,
) -> i64 {
    unsafe {
        let (Ok(pattern), Ok(input)) = (text(&pattern), text(&input)) else {
            return 0;
        };
        let Ok(regex) = regex::Regex::new(pattern) else {
            return 0;
        };
        let Some(out) = out.as_mut() else {
            return 0;
        };
        let outer_array_size = std::mem::size_of::<TyrusArray>();
        let element_ptr_size = std::mem::size_of::<TyrusString>();
        let mut match_arrays: Vec<TyrusArray> = Vec::new();
        for captures in regex.captures_iter(input) {
            let count = captures.len();
            let bytes = count.checked_mul(element_ptr_size).unwrap_or(0);
            let data = if bytes > 0 {
                let ptr = alloc_raw(bytes);
                if ptr.is_null() {
                    return 0;
                }
                ptr
            } else {
                std::ptr::null_mut()
            };
            let ptrs: Vec<TyrusString> = captures
                .iter()
                .map(|m| {
                    m.map_or(TyrusString::literal(""), |m| {
                        crate::strings::alloc_string(m.as_str())
                    })
                })
                .collect();
            if count > 0 {
                std::ptr::copy_nonoverlapping(ptrs.as_ptr(), data.cast(), count);
            }
            match_arrays.push(TyrusArray {
                data,
                length: count,
                capacity: count,
            });
        }
        let match_count = match_arrays.len();
        if match_count == 0 {
            *out = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return 0;
        }
        let outer_bytes = match_count.checked_mul(outer_array_size).unwrap_or(0);
        let outer_data = alloc_raw(outer_bytes);
        if outer_data.is_null() {
            return 0;
        }
        std::ptr::copy_nonoverlapping(match_arrays.as_ptr(), outer_data.cast(), match_count);
        // `TyrusArray` is a plain `repr(C)` struct with no `Drop` of its own, so
        // `match_arrays` dropping normally here only deallocates its own spine
        // (copied out just above) never touching what its elements' `data`
        // pointers point to. No `mem::forget` is needed, and the one that used
        // to sit here leaked that spine for no reason: nothing about this
        // vec's own elements required preserving.
        *out = TyrusArray {
            data: outer_data,
            length: match_count,
            capacity: match_count,
        };
        match_count as i64
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_replace(
    pattern: TyrusString,
    input: TyrusString,
    replacement: TyrusString,
) -> TyrusString {
    let (Ok(pattern), Ok(input), Ok(replacement)) =
        (text(&pattern), text(&input), text(&replacement))
    else {
        return TyrusString::null();
    };
    let Ok(regex) = regex::Regex::new(pattern) else {
        return TyrusString::null();
    };
    alloc_string(&regex.replace(input, replacement))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_split(
    pattern: TyrusString,
    input: TyrusString,
    out_array: *mut TyrusArray,
) {
    let set_empty = |arr: *mut TyrusArray| {
        if !arr.is_null() {
            unsafe {
                (*arr).data = std::ptr::null_mut();
                (*arr).length = 0;
                (*arr).capacity = 0;
            }
        }
    };
    let (Ok(pattern), Ok(input)) = (text(&pattern), text(&input)) else {
        return set_empty(out_array);
    };
    let Ok(regex) = regex::Regex::new(pattern) else {
        return set_empty(out_array);
    };
    let parts: Vec<&str> = regex.split(input).collect();
    let count = parts.len();
    if count == 0 {
        return set_empty(out_array);
    }
    let mut data: Vec<TyrusString> = Vec::with_capacity(count);
    for part in parts {
        data.push(alloc_string(part));
    }
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_new(pattern: TyrusString, flags: TyrusString) -> TyrusString {
    let Ok(pattern) = text(&pattern) else {
        panic_message("new RegExp pattern must be a valid string".to_string());
    };
    let Ok(flags) = text(&flags) else {
        panic_message("new RegExp flags must be a valid string".to_string());
    };
    let mut normalized_flags = String::new();
    for flag in flags.chars() {
        match flag {
            'i' | 'm' | 's' | 'x' => {
                if !normalized_flags.contains(flag) {
                    normalized_flags.push(flag);
                }
            }
            other => panic_message(format!(
                "`Regex` flag `{other}` is not supported; supported flags are `i`, `m`, `s`, and `x`"
            )),
        }
    }
    let compiled = if normalized_flags.is_empty() {
        pattern.to_owned()
    } else {
        format!("(?{normalized_flags}){pattern}")
    };
    if let Err(error) = regex::Regex::new(&compiled) {
        panic_message(format!(
            "`Regex` pattern is invalid: {error}. Look-around assertions and backreferences are not supported"
        ));
    }
    alloc_string(&compiled)
}

fn parse_regex_compiled(pattern: &str) -> (&str, &str) {
    if let Some(rest) = pattern.strip_prefix("(?")
        && let Some(end) = rest.find(')')
    {
        let flags = &rest[..end];
        if !flags.is_empty()
            && flags
                .bytes()
                .all(|b| matches!(b, b'i' | b'm' | b's' | b'x'))
        {
            return (&rest[end + 1..], flags);
        }
    }
    (pattern, "")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_get_source(pattern: TyrusString) -> TyrusString {
    let Ok(pattern) = text(&pattern) else {
        return TyrusString::null();
    };
    let (source, _) = parse_regex_compiled(pattern);
    alloc_string(source)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_get_flags(pattern: TyrusString) -> TyrusString {
    let Ok(pattern) = text(&pattern) else {
        return TyrusString::null();
    };
    let (_, flags) = parse_regex_compiled(pattern);
    alloc_string(flags)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_to_string(pattern: TyrusString) -> TyrusString {
    let Ok(pattern) = text(&pattern) else {
        return alloc_string("/(?:)/");
    };
    let (source, flags) = parse_regex_compiled(pattern);
    alloc_string(&format!("/{source}/{flags}"))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_regex_has_flag(pattern: TyrusString, flag_byte: i64) -> i64 {
    let Ok(pattern) = text(&pattern) else {
        return 0;
    };
    let flag = flag_byte as u8 as char;
    let (_, flags) = parse_regex_compiled(pattern);
    i64::from(flags.contains(flag))
}

fn alloc_raw(size: usize) -> *mut u8 {
    unsafe {
        let ptr = std::alloc::alloc(std::alloc::Layout::from_size_align(size, 1).unwrap());
        if ptr.is_null() {
            std::alloc::handle_alloc_error(std::alloc::Layout::from_size_align(size, 1).unwrap());
        }
        ptr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_len(s: TyrusString) -> i64 {
    if s.is_null() {
        return -1;
    }
    s.char_len() as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_not_blank(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_not_blank(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_trimmed(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_trimmed(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_lowercase(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_lowercase(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_uppercase(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_uppercase(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_email(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_email(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_url(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_url(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_uuid(s: TyrusString, version: i64) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    let version = (version != 0).then_some(version);
    i64::from(tyrus_validators::is_uuid(text, version))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_ip(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_ip(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_ipv4(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_ipv4(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_ipv6(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_ipv6(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_e164(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_e164(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_cep(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_cep(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_cpf(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_cpf(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_is_cnpj(s: TyrusString) -> i64 {
    let Ok(text) = text(&s) else {
        return 0;
    };
    i64::from(tyrus_validators::is_cnpj(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_index_of(
    s: TyrusString,
    search: TyrusString,
    from_index: i64,
) -> i64 {
    let (Ok(text), Ok(search)) = (text(&s), text(&search)) else {
        return -1;
    };
    core::index_of(text, s.char_len(), search, from_index)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_last_index_of(
    s: TyrusString,
    search: TyrusString,
    from_index: i64,
) -> i64 {
    let (Ok(text), Ok(search)) = (text(&s), text(&search)) else {
        return -1;
    };
    core::last_index_of(text, s.char_len(), search, from_index)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_slice(s: TyrusString, start: i64, end: i64) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(core::slice(text, s.char_len(), start, end))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_substring(
    s: TyrusString,
    start: i64,
    end: i64,
) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(core::substring(text, s.char_len(), start, end))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_trim(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(text.trim())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_trim_start(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(text.trim_start())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_trim_end(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(text.trim_end())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_lower(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&text.to_lowercase())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_upper(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&text.to_uppercase())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_replace(
    s: TyrusString,
    search: TyrusString,
    replacement: TyrusString,
) -> TyrusString {
    let (Ok(text), Ok(search), Ok(replacement)) = (text(&s), text(&search), text(&replacement))
    else {
        return TyrusString::null();
    };
    alloc_string(&core::replace_first(text, search, replacement))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_repeat(s: TyrusString, count: i64) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::repeat(text, count))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_pad_start(
    s: TyrusString,
    target_length: i64,
    pad_string: TyrusString,
) -> TyrusString {
    let (Ok(text), Ok(pad)) = (text(&s), text(&pad_string)) else {
        return TyrusString::null();
    };
    alloc_string(&core::pad_start(text, s.char_len(), target_length, pad))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_pad_end(
    s: TyrusString,
    target_length: i64,
    pad_string: TyrusString,
) -> TyrusString {
    let (Ok(text), Ok(pad)) = (text(&s), text(&pad_string)) else {
        return TyrusString::null();
    };
    alloc_string(&core::pad_end(text, s.char_len(), target_length, pad))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_concat(s: TyrusString, other: TyrusString) -> TyrusString {
    let (Ok(text), Ok(other)) = (text(&s), text(&other)) else {
        return TyrusString::null();
    };
    alloc_string(&core::concat(text, other))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_split(
    s: TyrusString,
    separator: TyrusString,
    out_array: *mut TyrusArray,
) {
    let set_empty = |arr: *mut TyrusArray| {
        if !arr.is_null() {
            unsafe {
                (*arr).data = std::ptr::null_mut();
                (*arr).length = 0;
                (*arr).capacity = 0;
            }
        }
    };
    let (Ok(text), Ok(separator)) = (text(&s), text(&separator)) else {
        return set_empty(out_array);
    };
    let parts = core::split(text, separator);
    let count = parts.len();
    if count == 0 {
        return set_empty(out_array);
    }
    let mut data: Vec<TyrusString> = Vec::with_capacity(count);
    for part in parts {
        data.push(alloc_string(part));
    }
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_slug_case(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::to_slug_case(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_dash_case(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::to_dash_case(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_pascal_case(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::to_pascal_case(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_camel_case(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::to_camel_case(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_to_snake_case(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::to_snake_case(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_truncate(s: TyrusString, max_length: i64) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::truncate(text, max_length))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_truncate_ellipsis(
    s: TyrusString,
    max_length: i64,
) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::truncate_ellipsis(text, max_length))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_strip_html(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::strip_html(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_escape_html(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::escape_html(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_escape_debug(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::escape_debug(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_url_encode(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::url_encode(text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_url_decode(s: TyrusString) -> TyrusString {
    let Ok(text) = text(&s) else {
        return TyrusString::null();
    };
    alloc_string(&core::url_decode(text))
}

fn scalar(value: u32) -> char {
    char::from_u32(value).unwrap_or(char::REPLACEMENT_CHARACTER)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_to_upper(value: u32) -> u32 {
    let mut uppercase = scalar(value).to_uppercase();
    match (uppercase.next(), uppercase.next()) {
        (Some(single), None) => single as u32,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_to_lower(value: u32) -> u32 {
    let mut lowercase = scalar(value).to_lowercase();
    match (lowercase.next(), lowercase.next()) {
        (Some(single), None) => single as u32,
        _ => value,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_is_digit(value: u32) -> bool {
    scalar(value).is_numeric()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_is_alphabetic(value: u32) -> bool {
    scalar(value).is_alphabetic()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_is_alphanumeric(value: u32) -> bool {
    scalar(value).is_alphanumeric()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_char_is_whitespace(value: u32) -> bool {
    scalar(value).is_whitespace()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_graphemes(s: TyrusString, out_array: *mut TyrusArray) {
    use unicode_segmentation::UnicodeSegmentation;

    let set_empty = |arr: *mut TyrusArray| {
        if !arr.is_null() {
            unsafe {
                (*arr).data = std::ptr::null_mut();
                (*arr).length = 0;
                (*arr).capacity = 0;
            }
        }
    };
    let Ok(text) = text(&s) else {
        return set_empty(out_array);
    };
    let clusters: Vec<&str> = text.graphemes(true).collect();
    if clusters.is_empty() {
        return set_empty(out_array);
    }
    let mut data: Vec<TyrusString> = Vec::with_capacity(clusters.len());
    for cluster in clusters {
        data.push(alloc_string(cluster));
    }
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_chars(s: TyrusString, out_array: *mut TyrusArray) {
    let Ok(text) = text(&s) else {
        unsafe {
            (*out_array).data = std::ptr::null_mut();
            (*out_array).length = 0;
            (*out_array).capacity = 0;
        }
        return;
    };
    let mut data: Vec<u32> = text.chars().map(|character| character as u32).collect();
    let ptr = data.as_mut_ptr();
    let len = data.len();
    let cap = data.capacity();
    std::mem::forget(data);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_string_alloc(bytes: usize) -> *mut u8 {
    TyrusString::alloc_buffer(bytes)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_string_share(value: TyrusString) -> TyrusString {
    value.share()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_release_buffer(text: *const u8) {
    unsafe { TyrusString::release_buffer(text) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_string_release(value: TyrusString) {
    unsafe { value.release() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cstr(value: &str) -> TyrusString {
        unsafe { TyrusString::lend(value) }.expect("test text is within the limit")
    }

    unsafe fn take_string(value: TyrusString) -> String {
        assert!(!value.is_null(), "runtime returned a null string");
        let owned = unsafe { value.as_str() }.to_owned();
        unsafe { value.release() };
        owned
    }

    #[test]
    fn indexes_by_code_point_rather_than_utf8_byte() {
        let text = cstr("héllo");
        unsafe {
            assert_eq!(tyrus_string_len(text.borrow()), 5);
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 1)), "é");
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 2)), "l");
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 4)), "o");
            assert_eq!(tyrus_string_char_code_at(text.borrow(), 1), 233);
            assert_eq!(take_string(tyrus_string_slice(text.borrow(), 1, 3)), "él");
            assert_eq!(
                take_string(tyrus_string_substring(text.borrow(), 1, 3)),
                "él"
            );
            assert_eq!(tyrus_string_index_of(text.borrow(), cstr("llo"), 0), 2);
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), cstr("l"), INDEX_UNSET),
                3
            );
        }
    }

    #[test]
    fn treats_an_astral_character_as_one_position() {
        let text = cstr("a🚀b");
        unsafe {
            assert_eq!(tyrus_string_len(text.borrow()), 3);
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 1)), "🚀");
            assert_eq!(take_string(tyrus_string_slice(text.borrow(), 1, 2)), "🚀");
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 2)), "b");
        }
    }

    #[test]
    fn clamps_out_of_range_indexes_instead_of_panicking() {
        let text = cstr("héllo");
        unsafe {
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 5)), "");
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), 99)), "");
            assert_eq!(tyrus_string_char_code_at(text.borrow(), 99), -1);
            assert_eq!(take_string(tyrus_string_char_at(text.borrow(), -1)), "o");
            assert_eq!(take_string(tyrus_string_slice(text.borrow(), -2, 99)), "lo");
            assert_eq!(take_string(tyrus_string_slice(text.borrow(), 3, 1)), "");
        }
    }

    #[test]
    fn an_omitted_end_argument_reaches_the_end_of_the_string() {
        let text = cstr("hello");
        unsafe {
            assert_eq!(
                take_string(tyrus_string_slice(text.borrow(), 1, INDEX_UNSET)),
                "ello"
            );
            assert_eq!(
                take_string(tyrus_string_substring(text.borrow(), 1, INDEX_UNSET)),
                "ello"
            );
        }
    }

    #[test]
    fn a_negative_last_index_of_start_means_position_zero() {
        let text = cstr("abcabc");
        let needle = cstr("a");
        unsafe {
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), needle.borrow(), INDEX_UNSET),
                3
            );
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), needle.borrow(), -1),
                0
            );
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), needle.borrow(), -99),
                0
            );
            assert_eq!(tyrus_string_last_index_of(text.borrow(), cstr("b"), -1), -1);
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), needle.borrow(), 0),
                0
            );
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), needle.borrow(), 2),
                0
            );
            assert_eq!(tyrus_string_last_index_of(text.borrow(), cstr(""), -1), 0);
            assert_eq!(
                tyrus_string_last_index_of(text.borrow(), cstr(""), INDEX_UNSET),
                6
            );
        }
    }

    #[test]
    fn pads_by_code_point() {
        unsafe {
            assert_eq!(
                take_string(tyrus_string_pad_start(cstr("é"), 3, cstr("→"))),
                "→→é"
            );
            assert_eq!(
                take_string(tyrus_string_pad_end(cstr("é"), 3, cstr("→"))),
                "é→→"
            );
            assert_eq!(
                take_string(tyrus_string_pad_start(cstr("héllo"), 3, cstr("0"))),
                "héllo"
            );
        }
    }

    unsafe fn free_regex_exec_all_result(out: &TyrusArray) {
        unsafe {
            let inner_arrays = out.data as *mut TyrusArray;
            for match_index in 0..out.length {
                let inner = &*inner_arrays.add(match_index);
                let slots = inner.data as *mut TyrusString;
                for slot_index in 0..inner.length {
                    std::mem::replace(&mut *slots.add(slot_index), TyrusString::null()).release();
                }
                if !inner.data.is_null() {
                    let size = inner.length * std::mem::size_of::<TyrusString>();
                    std::alloc::dealloc(
                        inner.data,
                        std::alloc::Layout::from_size_align(size, 1).unwrap(),
                    );
                }
            }
            if !out.data.is_null() {
                let size = out.length * std::mem::size_of::<TyrusArray>();
                std::alloc::dealloc(
                    out.data,
                    std::alloc::Layout::from_size_align(size, 1).unwrap(),
                );
            }
        }
    }

    /// Every index in the string API counts characters. Byte indices used to
    /// leak through: `"héllo".length` answered 6, and slicing across a
    /// multi-byte character panicked instead of returning the characters.
    #[test]
    fn string_indices_count_characters_not_bytes() {
        let accented = cstr("héllo");
        let launch = cstr("a🚀b");
        unsafe {
            assert_eq!(tyrus_string_len(accented.borrow()), 5);
            assert_eq!(tyrus_string_len(launch.borrow()), 3);

            assert_eq!(owned(tyrus_string_char_at(accented.borrow(), 1)), "é");
            assert_eq!(owned(tyrus_string_char_at(launch.borrow(), 1)), "🚀");
            assert_eq!(owned(tyrus_string_char_at(accented.borrow(), -1)), "o");
            assert_eq!(tyrus_string_char_code_at(launch.borrow(), 1), '🚀' as i32);

            assert_eq!(owned(tyrus_string_slice(accented.borrow(), 0, 2)), "hé");
            assert_eq!(owned(tyrus_string_slice(launch.borrow(), 1, 2)), "🚀");
            assert_eq!(owned(tyrus_string_slice(accented.borrow(), -2, 5)), "lo");
            assert_eq!(owned(tyrus_string_substring(accented.borrow(), 1, 3)), "él");

            let l = cstr("l");
            assert_eq!(tyrus_string_index_of(accented.borrow(), l.borrow(), 0), 2);
            assert_eq!(
                tyrus_string_last_index_of(accented.borrow(), l.borrow(), 4),
                3
            );
            let b = cstr("b");
            assert_eq!(tyrus_string_index_of(launch.borrow(), b.borrow(), 0), 2);

            let dot = cstr(".");
            assert_eq!(
                owned(tyrus_string_pad_start(accented.borrow(), 7, dot.borrow())),
                "..héllo"
            );
            assert_eq!(
                owned(tyrus_string_pad_end(launch.borrow(), 5, dot.borrow())),
                "a🚀b.."
            );
            let rocket = cstr("🚀");
            assert_eq!(
                owned(tyrus_string_pad_start(
                    accented.borrow(),
                    7,
                    rocket.borrow()
                )),
                "🚀🚀héllo"
            );
        }
    }

    #[test]
    fn regex_exec_all_returns_all_matches() {
        let pattern = cstr(r"\w+");
        let input = cstr("hello world");
        let mut out = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let result = unsafe { tyrus_regex_exec_all(pattern.borrow(), input.borrow(), &mut out) };
        assert!(result > 0, "expected matches, got {result}");
        assert_eq!(out.length, 2, "expected 2 matches");
        unsafe {
            let inner: &TyrusArray = &*(out.data as *const TyrusArray);
            assert_eq!(inner.length, 1, "each match has 1 group");
            let s = (*(inner.data as *const TyrusString)).as_str();
            assert_eq!(s, "hello");
            let inner2: &TyrusArray = &*((out.data as *const TyrusArray).add(1));
            assert_eq!(inner2.length, 1);
            let s2 = (*(inner2.data as *const TyrusString)).as_str();
            assert_eq!(s2, "world");
            free_regex_exec_all_result(&out);
        }
    }

    fn owned(value: TyrusString) -> String {
        let text = unsafe { value.as_str() }.to_string();
        unsafe { value.release() };
        text
    }

    #[test]
    fn formats_full_width_integers_without_an_intermediate_string() {
        assert_eq!(owned(tyrus_format_i128(i128::MIN)), i128::MIN.to_string());
        assert_eq!(owned(tyrus_format_i128(i128::MAX)), i128::MAX.to_string());
        assert_eq!(owned(tyrus_format_u128(u128::MAX)), u128::MAX.to_string());
    }

    #[test]
    fn format_f64_precision_pads_and_truncates_decimal_places() {
        assert_eq!(owned(tyrus_format_f64_precision(12.34567, 2)), "12.35");
        assert_eq!(owned(tyrus_format_f64_precision(3.0, 2)), "3.00");
        assert_eq!(owned(tyrus_format_f64_precision(12.34567, 0)), "12");
    }

    #[test]
    fn formats_special_floating_point_values_with_tyrus_literal_spellings() {
        assert_eq!(owned(tyrus_format_f64(f64::INFINITY)), "Infinity");
        assert_eq!(owned(tyrus_format_f64(f64::NEG_INFINITY)), "-Infinity");
        assert_eq!(owned(tyrus_format_f64(f64::NAN)), "NaN");
        assert_eq!(owned(tyrus_format_f32(f32::INFINITY)), "Infinity");
        assert_eq!(owned(tyrus_format_f32(f32::NEG_INFINITY)), "-Infinity");
        assert_eq!(owned(tyrus_format_f32(f32::NAN)), "NaN");
    }

    #[test]
    fn format_i128_padded_pads_with_leading_zeros() {
        assert_eq!(owned(tyrus_format_i128_padded(7, 4)), "0007");
        assert_eq!(owned(tyrus_format_i128_padded(-7, 4)), "-007");
        assert_eq!(
            owned(tyrus_format_i128_padded(123456789, 12)),
            "000123456789"
        );
        assert_eq!(owned(tyrus_format_i128_padded(12345, 2)), "12345");
    }

    #[test]
    fn format_u128_padded_pads_with_leading_zeros() {
        assert_eq!(owned(tyrus_format_u128_padded(7, 4)), "0007");
        assert_eq!(owned(tyrus_format_u128_padded(12345, 2)), "12345");
    }

    #[test]
    fn regex_exec_all_with_capture_groups() {
        let pattern = cstr(r"(\w+)=(\w+)");
        let input = cstr("a=1 b=2");
        let mut out = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let result = unsafe { tyrus_regex_exec_all(pattern.borrow(), input.borrow(), &mut out) };
        assert!(result > 0, "expected matches, got {result}");
        assert_eq!(out.length, 2, "expected 2 matches");
        unsafe {
            let inner: &TyrusArray = &*(out.data as *const TyrusArray);
            assert_eq!(inner.length, 3, "match has 3 groups");
            let ptrs = inner.data as *const TyrusString;
            let s0 = (*ptrs).as_str();
            assert_eq!(s0, "a=1");
            let s1 = (*ptrs.add(1)).as_str();
            assert_eq!(s1, "a");
            let s2 = (*ptrs.add(2)).as_str();
            assert_eq!(s2, "1");
            free_regex_exec_all_result(&out);
        }
    }

    #[test]
    fn regex_exec_all_no_match_returns_zero() {
        let pattern = cstr(r"\d+");
        let input = cstr("hello world");
        let mut out = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let result = unsafe { tyrus_regex_exec_all(pattern.borrow(), input.borrow(), &mut out) };
        assert_eq!(result, 0);
        assert_eq!(out.length, 0);
    }

    #[test]
    fn regex_replaces_first_match() {
        let pattern = cstr(r"\d+");
        let input = cstr("abc123def456");
        let replacement = cstr("X");
        let result =
            unsafe { tyrus_regex_replace(pattern.borrow(), input.borrow(), replacement.borrow()) };
        let s = unsafe { result.as_str() };
        assert_eq!(s, "abcXdef456");
        unsafe { result.release() };
    }

    #[test]
    fn regex_replaces_no_match_returns_original() {
        let pattern = cstr(r"\d+");
        let input = cstr("abcdef");
        let replacement = cstr("X");
        let result =
            unsafe { tyrus_regex_replace(pattern.borrow(), input.borrow(), replacement.borrow()) };
        let s = unsafe { result.as_str() };
        assert_eq!(s, "abcdef");
        unsafe { result.release() };
    }

    #[test]
    fn regex_search_returns_first_match_position() {
        let pattern = cstr(r"\d+");
        let input = cstr("abc123def");
        let result = unsafe { tyrus_string_search_regex(input.borrow(), pattern.borrow()) };
        assert_eq!(result, 3);
    }

    #[test]
    fn regex_search_no_match_returns_negative_one() {
        let pattern = cstr(r"\d+");
        let input = cstr("abcdef");
        let result = unsafe { tyrus_string_search_regex(input.borrow(), pattern.borrow()) };
        assert_eq!(result, -1);
    }

    #[test]
    fn regex_splits_by_pattern() {
        let pattern = cstr(r"\s+");
        let input = cstr("hello   world  foo");
        let mut out = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { tyrus_regex_split(pattern.borrow(), input.borrow(), &mut out) };
        assert_eq!(out.length, 3);
        unsafe {
            let ptrs = out.data as *const TyrusString;
            let s0 = (*ptrs).as_str();
            assert_eq!(s0, "hello");
            let s1 = (*ptrs.add(1)).as_str();
            assert_eq!(s1, "world");
            let s2 = (*ptrs.add(2)).as_str();
            assert_eq!(s2, "foo");
            let reclaimed: Vec<TyrusString> =
                Vec::from_raw_parts(out.data as *mut TyrusString, out.length, out.capacity);
            for slot in reclaimed {
                slot.release();
            }
        }
    }

    #[test]
    fn regex_new_compiles_pattern_and_flags() {
        let pattern = cstr("hello");
        let flags = cstr("im");
        let result = unsafe { tyrus_regex_new(pattern.borrow(), flags.borrow()) };
        let s = unsafe { result.as_str() };
        assert_eq!(s, "(?im)hello");
        unsafe { result.release() };
    }

    #[test]
    fn regex_new_deduplicates_flags() {
        let pattern = cstr("hello");
        let flags = cstr("iim");
        let result = unsafe { tyrus_regex_new(pattern.borrow(), flags.borrow()) };
        let s = unsafe { result.as_str() };
        assert_eq!(s, "(?im)hello");
        unsafe { result.release() };
    }

    #[test]
    fn validates_string_shape_helpers() {
        let email = cstr("ada@example.com");
        let invalid_email = cstr("ada.example.com");
        let invalid_email_double_at = cstr("ada@@example.com");
        let invalid_email_bad_domain = cstr("ada@example..com");
        let url = cstr("https://example.com/docs");
        let invalid_url = cstr("mailto:ada@example.com");
        let slug = cstr("release-2026");
        let invalid_slug = cstr("Release 2026");
        let slug_pattern = cstr("^[a-z0-9-]+$");
        let invalid_pattern = cstr("(?=release).*");
        unsafe {
            assert_eq!(tyrus_string_is_email(email.borrow()), 1);
            assert_eq!(tyrus_string_is_email(invalid_email.borrow()), 0);
            assert_eq!(tyrus_string_is_email(invalid_email_double_at.borrow()), 0);
            assert_eq!(tyrus_string_is_email(invalid_email_bad_domain.borrow()), 0);
            assert_eq!(tyrus_string_is_url(url.borrow()), 1);
            assert_eq!(tyrus_string_is_url(invalid_url.borrow()), 0);
            assert_eq!(
                tyrus_string_matches_regex(slug.borrow(), slug_pattern.borrow()),
                1
            );
            assert_eq!(
                tyrus_string_matches_regex(invalid_slug.borrow(), slug_pattern.borrow()),
                0
            );
            assert_eq!(
                tyrus_string_matches_regex(slug.borrow(), invalid_pattern.borrow()),
                0
            );
        }
    }

    #[test]
    fn validates_identifier_and_network_helpers() {
        let uuid = cstr("01890f3d-7d2b-7cc8-98f1-97a7c8f1a111");
        let ipv4 = cstr("127.0.0.1");
        let ipv6 = cstr("::1");
        unsafe {
            assert_eq!(tyrus_string_is_uuid(uuid.borrow(), 7), 1);
            assert_eq!(tyrus_string_is_uuid(uuid.borrow(), 4), 0);
            assert_eq!(tyrus_string_is_ip(ipv4.borrow()), 1);
            assert_eq!(tyrus_string_is_ipv4(ipv4.borrow()), 1);
            assert_eq!(tyrus_string_is_ipv6(ipv4.borrow()), 0);
            assert_eq!(tyrus_string_is_ipv6(ipv6.borrow()), 1);
        }
    }

    #[test]
    fn validates_localized_format_helpers() {
        let cep = cstr("01001-000");
        let cpf = cstr("123.456.789-09");
        let cpf_compact = cstr("12345678909");
        let invalid_cpf_digits = cstr("123.456.789-00");
        let invalid_cpf_repeated = cstr("111.111.111-11");
        let cnpj = cstr("12.345.678/0001-95");
        let cnpj_compact = cstr("12345678000195");
        let alphanumeric_cnpj = cstr("AB.CDE.F12/3456-80");
        let alphanumeric_cnpj_compact = cstr("ABCDEF12345680");
        let invalid_cnpj_digits = cstr("AB.CDE.F12/3456-81");
        let invalid_cnpj_suffix = cstr("AB.CDE.F12/3456-AA");
        unsafe {
            assert_eq!(tyrus_string_is_cep(cep.borrow()), 1);
            assert_eq!(tyrus_string_is_cpf(cpf.borrow()), 1);
            assert_eq!(tyrus_string_is_cpf(cpf_compact.borrow()), 1);
            assert_eq!(tyrus_string_is_cpf(invalid_cpf_digits.borrow()), 0);
            assert_eq!(tyrus_string_is_cpf(invalid_cpf_repeated.borrow()), 0);
            assert_eq!(tyrus_string_is_cnpj(cnpj.borrow()), 1);
            assert_eq!(tyrus_string_is_cnpj(cnpj_compact.borrow()), 1);
            assert_eq!(tyrus_string_is_cnpj(alphanumeric_cnpj.borrow()), 1);
            assert_eq!(tyrus_string_is_cnpj(alphanumeric_cnpj_compact.borrow()), 1);
            assert_eq!(tyrus_string_is_cnpj(invalid_cnpj_digits.borrow()), 0);
            assert_eq!(tyrus_string_is_cnpj(invalid_cnpj_suffix.borrow()), 0);
        }
    }

    #[test]
    fn count_occurrences_counts_non_overlapping_matches() {
        let banana = cstr("banana");
        let an = cstr("an");
        let xx = cstr("xx");
        let empty_needle = cstr("");
        let empty_haystack = cstr("");
        unsafe {
            assert_eq!(
                tyrus_string_count_occurrences(banana.borrow(), an.borrow()),
                2
            );
            assert_eq!(
                tyrus_string_count_occurrences(banana.borrow(), xx.borrow()),
                0
            );
            assert_eq!(
                tyrus_string_count_occurrences(banana.borrow(), empty_needle.borrow()),
                0
            );
            assert_eq!(
                tyrus_string_count_occurrences(empty_haystack.borrow(), an.borrow()),
                0
            );
        }
    }

    #[test]
    fn count_occurrences_does_not_count_overlapping_matches() {
        let aaa = cstr("aaa");
        let aa = cstr("aa");
        unsafe {
            assert_eq!(tyrus_string_count_occurrences(aaa.borrow(), aa.borrow()), 1);
        }
    }
}
