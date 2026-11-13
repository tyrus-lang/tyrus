//! Native UTF-8, UTF-16, hexadecimal, Base64, Base32, and radix conversion for `tyrus:encoding`.
//!
//! The codec rules live in the pure `tyrus-encoding` crate, where they are
//! formally verified. This module is the ABI around them: it reads the values
//! generated code passes, writes results into runtime-owned buffers, and maps a
//! rejected input to the stable `EncodingError` code the program can catch.
//!
//! Only malformed input is recoverable. A null buffer with a non-zero length, a
//! null output slot, or an allocation failure means the compiler or the machine
//! broke a contract, so those abort through `tyrus_panic` instead of surfacing
//! as an `EncodingError` that a caller could mistake for bad data. Encoders and
//! lossy decoders therefore never throw.

use super::*;
use std::cell::Cell;
use std::ffi::CStr;
use std::ptr;
use tyrus_encoding::RadixFailure;
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[repr(C)]
pub struct EncodingErrorTaken {
    pub code: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
pub struct EncodingResultFfi<T> {
    pub tag: u8,
    pub payload: EncodingResultPayload<T>,
}

#[repr(C)]
pub union EncodingResultPayload<T> {
    pub ok: std::mem::ManuallyDrop<T>,
    pub err: std::mem::ManuallyDrop<EncodingErrorTaken>,
    words: [u64; 4],
}

const _: () = {
    assert!(std::mem::size_of::<EncodingErrorTaken>() == 2 * std::mem::size_of::<TyrusString>());
    assert!(std::mem::size_of::<EncodingResultFfi<TyrusString>>() == 40);
    assert!(std::mem::size_of::<EncodingResultFfi<TyrusArray>>() == 40);
    assert!(std::mem::size_of::<EncodingResultFfi<i64>>() == 40);
    assert!(std::mem::size_of::<EncodingResultFfi<u64>>() == 40);
    assert!(std::mem::offset_of!(EncodingResultFfi<TyrusString>, payload) == 8);
};

thread_local! {
    static LAST_ENCODING_ERROR: Cell<Option<(&'static CStr, &'static CStr)>> = const { Cell::new(None) };
}

fn set_encoding_error(code: &'static CStr, message: &'static CStr) {
    LAST_ENCODING_ERROR.with(|slot| slot.set(Some((code, message))));
}

fn clear_last_encoding_error() {
    LAST_ENCODING_ERROR.with(|slot| slot.set(None));
}

unsafe fn write_encoding_result<T>(out: *mut EncodingResultFfi<T>, value: T) {
    if out.is_null() {
        encoding_abort(c"tyrus runtime error: `tyrus:encoding` received a null Result output");
    }
    let error = LAST_ENCODING_ERROR.with(std::cell::Cell::take);
    let result = if let Some((code, message)) = error {
        EncodingResultFfi {
            tag: 1,
            payload: EncodingResultPayload {
                err: std::mem::ManuallyDrop::new(EncodingErrorTaken {
                    code: TyrusString::literal(code.to_str().expect("static UTF-8 code")),
                    message: TyrusString::literal(message.to_str().expect("static UTF-8 message")),
                }),
            },
        }
    } else {
        EncodingResultFfi {
            tag: 0,
            payload: EncodingResultPayload {
                ok: std::mem::ManuallyDrop::new(value),
            },
        }
    };
    unsafe { out.write(result) };
}

macro_rules! encoding_error {
    ($code:expr_2021, $message:expr_2021, $fallback:expr_2021) => {{
        set_encoding_error($code, $message);
        return $fallback;
    }};
    ($code:expr_2021, $message:expr_2021) => {{
        set_encoding_error($code, $message);
        return;
    }};
}

fn encoding_abort(message: &'static CStr) -> ! {
    // SAFETY: the message is a static C string literal.
    unsafe { crate::tyrus_panic(message.as_ptr()) }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_encoding_clear_error() {
    LAST_ENCODING_ERROR.with(|slot| slot.set(None));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_take_error(out: *mut EncodingErrorTaken) {
    LAST_ENCODING_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take()
            && let Some(out) = unsafe { out.as_mut() }
        {
            out.code = TyrusString::literal(code.to_str().expect("a static error code is UTF-8"));
            out.message =
                TyrusString::literal(message.to_str().expect("a static error message is UTF-8"));
        }
    });
}

unsafe fn input_slice<'a, T>(data: *const T, length: usize) -> &'a [T] {
    if length == 0 {
        return &[];
    }
    if data.is_null() {
        encoding_abort(
            c"tyrus runtime error: `tyrus:encoding` received a null buffer with a non-zero length",
        );
    }
    if length.checked_mul(std::mem::size_of::<T>()).is_none() {
        encoding_abort(
            c"tyrus runtime error: `tyrus:encoding` received a buffer length that overflows",
        );
    }
    unsafe { std::slice::from_raw_parts(data, length) }
}

unsafe fn write_array<T: Copy>(out: *mut TyrusArray, values: &[T]) {
    let Some(out) = (unsafe { out.as_mut() }) else {
        encoding_abort(c"tyrus runtime error: `tyrus:encoding` received a null output array");
    };
    if values.is_empty() {
        *out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        return;
    }
    let bytes = std::mem::size_of_val(values);
    let allocation = unsafe { malloc(bytes) }.cast::<T>();
    if allocation.is_null() {
        encoding_abort(c"tyrus runtime error: out of memory while allocating an encoding result");
    }
    unsafe { ptr::copy_nonoverlapping(values.as_ptr(), allocation, values.len()) };
    *out = TyrusArray {
        data: allocation.cast(),
        length: values.len(),
        capacity: values.len(),
    };
}

unsafe fn input_string(value: &TyrusString) -> &str {
    if value.is_null() {
        encoding_abort(c"tyrus runtime error: `tyrus:encoding` received an absent string");
    }
    unsafe { value.as_str() }
}

fn utf16_string(value: Option<String>) -> TyrusString {
    match value {
        Some(value) => crate::strings::alloc_string(&value),
        None => encoding_error!(
            c"ERR_ENCODING_INVALID_UTF16",
            c"input is not valid UTF-16",
            TyrusString::null()
        ),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf8_encode(value: TyrusString, out: *mut TyrusArray) {
    let value = unsafe { input_string(&value) };
    unsafe { write_array(out, value.as_bytes()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf8_decode(data: *const u8, length: usize) -> TyrusString {
    let bytes = unsafe { input_slice(data, length) };
    match std::str::from_utf8(bytes) {
        Ok(text) => crate::strings::alloc_string(text),
        Err(_) => encoding_error!(
            c"ERR_ENCODING_INVALID_UTF8",
            c"input is not valid UTF-8",
            TyrusString::null()
        ),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf8_decode_lossy(
    data: *const u8,
    length: usize,
) -> TyrusString {
    let bytes = unsafe { input_slice(data, length) };
    crate::strings::alloc_string(&String::from_utf8_lossy(bytes))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_encode(value: TyrusString, out: *mut TyrusArray) {
    let value = unsafe { input_string(&value) };
    let units: Vec<u16> = value.encode_utf16().collect();
    unsafe { write_array(out, &units) };
}

unsafe fn encode_utf16_bytes(value: TyrusString, out: *mut TyrusArray, little_endian: bool) {
    let value = unsafe { input_string(&value) };
    let bytes: Vec<u8> = value
        .encode_utf16()
        .flat_map(|unit| {
            if little_endian {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            }
        })
        .collect();
    unsafe { write_array(out, &bytes) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_encode_le(value: TyrusString, out: *mut TyrusArray) {
    unsafe { encode_utf16_bytes(value, out, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_encode_be(value: TyrusString, out: *mut TyrusArray) {
    unsafe { encode_utf16_bytes(value, out, false) }
}

unsafe fn decode_utf16_units(data: *const u16, length: usize, lossy: bool) -> TyrusString {
    let units = unsafe { input_slice(data, length) };
    utf16_string(tyrus_encoding::decode_utf16(units, lossy))
}

unsafe fn decode_utf16_bytes(
    data: *const u8,
    length: usize,
    little_endian: bool,
    lossy: bool,
) -> TyrusString {
    let bytes = unsafe { input_slice(data, length) };
    utf16_string(
        tyrus_encoding::utf16_from_bytes(bytes, little_endian)
            .and_then(|units| tyrus_encoding::decode_utf16(&units, lossy)),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode(
    data: *const u16,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_units(data, length, false) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode_lossy(
    data: *const u16,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_units(data, length, true) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode_le(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_bytes(data, length, true, false) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode_le_lossy(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_bytes(data, length, true, true) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode_be(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_bytes(data, length, false, false) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_utf16_decode_be_lossy(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe { decode_utf16_bytes(data, length, false, true) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_hex_encode(data: *const u8, length: usize) -> TyrusString {
    let bytes = unsafe { input_slice(data, length) };
    crate::strings::alloc_string(&tyrus_encoding::hex_encode(bytes))
}

unsafe fn decode_text_to_bytes(
    value: TyrusString,
    out: *mut TyrusArray,
    decode: impl FnOnce(&[u8]) -> Option<Vec<u8>>,
    code: &'static CStr,
    message: &'static CStr,
) {
    let text = unsafe { input_string(&value) };
    match decode(text.as_bytes()) {
        Some(bytes) => unsafe { write_array(out, &bytes) },
        None => encoding_error!(code, message),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_hex_decode(value: TyrusString, out: *mut TyrusArray) {
    unsafe {
        decode_text_to_bytes(
            value,
            out,
            tyrus_encoding::hex_decode,
            c"ERR_ENCODING_INVALID_HEX",
            c"input is not valid hexadecimal",
        );
    }
}

unsafe fn encode_bytes_to_string(
    data: *const u8,
    length: usize,
    encode: impl FnOnce(&[u8]) -> String,
) -> TyrusString {
    let bytes = unsafe { input_slice(data, length) };
    crate::strings::alloc_string(&encode(bytes))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base64_encode(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe {
        encode_bytes_to_string(data, length, |bytes| {
            tyrus_encoding::base64_encode(bytes, false)
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base64url_encode(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe {
        encode_bytes_to_string(data, length, |bytes| {
            tyrus_encoding::base64_encode(bytes, true)
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base32_encode(
    data: *const u8,
    length: usize,
) -> TyrusString {
    unsafe { encode_bytes_to_string(data, length, tyrus_encoding::base32_encode) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base64_decode(value: TyrusString, out: *mut TyrusArray) {
    unsafe {
        decode_text_to_bytes(
            value,
            out,
            |text| tyrus_encoding::base64_decode(text, false),
            c"ERR_ENCODING_INVALID_BASE64",
            c"input is not valid base64",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base64url_decode(value: TyrusString, out: *mut TyrusArray) {
    unsafe {
        decode_text_to_bytes(
            value,
            out,
            |text| tyrus_encoding::base64_decode(text, true),
            c"ERR_ENCODING_INVALID_BASE64",
            c"input is not valid base64url",
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_base32_decode(value: TyrusString, out: *mut TyrusArray) {
    unsafe {
        decode_text_to_bytes(
            value,
            out,
            tyrus_encoding::base32_decode,
            c"ERR_ENCODING_INVALID_BASE32",
            c"input is not valid base32",
        );
    }
}

fn radix_failure_error(failure: RadixFailure) {
    let (code, message) = match failure {
        RadixFailure::InvalidBase => (
            c"ERR_ENCODING_INVALID_RADIX",
            c"base must be 2, 8, 10, or 16",
        ),
        RadixFailure::InvalidInput => (
            c"ERR_ENCODING_INVALID_RADIX",
            c"input is not a valid base 2, 8, 10, or 16 integer",
        ),
        RadixFailure::Overflow => (
            c"ERR_ENCODING_INPUT_TOO_LARGE",
            c"value overflows the target integer type",
        ),
    };
    set_encoding_error(code, message);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_radix_format(value: i64, base: i32) -> TyrusString {
    match tyrus_encoding::radix_format_i64(value, base) {
        Ok(text) => crate::strings::alloc_string(&text),
        Err(_) => {
            encoding_abort(c"tyrus runtime error: `radix.format` base must be 2, 8, 10, or 16")
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_radix_parse_i64(value: TyrusString, base: i32) -> i64 {
    let text = unsafe { input_string(&value) };
    tyrus_encoding::radix_parse_i64(text.as_bytes(), base).unwrap_or_else(|failure| {
        radix_failure_error(failure);
        0
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_radix_parse_u64(value: TyrusString, base: i32) -> u64 {
    let text = unsafe { input_string(&value) };
    tyrus_encoding::radix_parse_u64(text.as_bytes(), base).unwrap_or_else(|failure| {
        radix_failure_error(failure);
        0
    })
}

macro_rules! strict_string_decode_result {
    ($wrapper:ident, $raw:ident, $unit:ty) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $wrapper(
            data: *const $unit,
            length: usize,
            out: *mut EncodingResultFfi<TyrusString>,
        ) {
            clear_last_encoding_error();
            let value = unsafe { $raw(data, length) };
            unsafe { write_encoding_result(out, value) };
        }
    };
}

strict_string_decode_result!(
    tyrus_encoding_utf8_decode_result,
    tyrus_encoding_utf8_decode,
    u8
);
strict_string_decode_result!(
    tyrus_encoding_utf16_decode_result,
    tyrus_encoding_utf16_decode,
    u16
);
strict_string_decode_result!(
    tyrus_encoding_utf16_decode_le_result,
    tyrus_encoding_utf16_decode_le,
    u8
);
strict_string_decode_result!(
    tyrus_encoding_utf16_decode_be_result,
    tyrus_encoding_utf16_decode_be,
    u8
);

macro_rules! strict_array_decode_result {
    ($wrapper:ident, $raw:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $wrapper(
            value: TyrusString,
            out: *mut EncodingResultFfi<TyrusArray>,
        ) {
            clear_last_encoding_error();
            let mut bytes = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            unsafe { $raw(value, &mut bytes) };
            unsafe { write_encoding_result(out, bytes) };
        }
    };
}

strict_array_decode_result!(tyrus_encoding_hex_decode_result, tyrus_encoding_hex_decode);
strict_array_decode_result!(
    tyrus_encoding_base64_decode_result,
    tyrus_encoding_base64_decode
);
strict_array_decode_result!(
    tyrus_encoding_base64url_decode_result,
    tyrus_encoding_base64url_decode
);
strict_array_decode_result!(
    tyrus_encoding_base32_decode_result,
    tyrus_encoding_base32_decode
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_radix_parse_i64_result(
    value: TyrusString,
    base: i32,
    out: *mut EncodingResultFfi<i64>,
) {
    clear_last_encoding_error();
    let parsed = unsafe { tyrus_encoding_radix_parse_i64(value, base) };
    unsafe { write_encoding_result(out, parsed) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_encoding_radix_parse_u64_result(
    value: TyrusString,
    base: i32,
    out: *mut EncodingResultFfi<u64>,
) {
    clear_last_encoding_error();
    let parsed = unsafe { tyrus_encoding_radix_parse_u64(value, base) };
    unsafe { write_encoding_result(out, parsed) };
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        fn free(pointer: *mut c_void);
    }

    unsafe fn owned(value: TyrusString) -> String {
        let text = unsafe { value.as_str() }.to_owned();
        unsafe { value.release() };
        text
    }

    fn take_code() -> String {
        let mut error = EncodingErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { tyrus_encoding_take_error(&mut error) };
        unsafe { error.code.as_str() }.to_owned()
    }

    #[test]
    fn radix_result_wrapper_returns_ok_and_structured_err() {
        let value = crate::strings::alloc_string("-ff");
        let mut result = std::mem::MaybeUninit::<EncodingResultFfi<i64>>::uninit();
        unsafe { tyrus_encoding_radix_parse_i64_result(value.borrow(), 16, result.as_mut_ptr()) };
        unsafe { value.release() };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 0);
        assert_eq!(unsafe { *result.payload.ok }, -255);

        let value = crate::strings::alloc_string("-1");
        let mut result = std::mem::MaybeUninit::<EncodingResultFfi<u64>>::uninit();
        unsafe { tyrus_encoding_radix_parse_u64_result(value.borrow(), 10, result.as_mut_ptr()) };
        unsafe { value.release() };
        let result = unsafe { result.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe { &*result.payload.err };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_ENCODING_INVALID_RADIX");
        assert_eq!(
            unsafe { error.message.as_str() },
            "input is not a valid base 2, 8, 10, or 16 integer"
        );
    }

    #[test]
    fn decodes_utf16_units_and_explicit_byte_order() {
        let units = [0x0041u16, 0xd83d, 0xde00];
        let le = [0x41u8, 0x00, 0x3d, 0xd8, 0x00, 0xde];
        let be = [0x00u8, 0x41, 0xd8, 0x3d, 0xde, 0x00];
        assert_eq!(
            unsafe { owned(tyrus_encoding_utf16_decode(units.as_ptr(), units.len())) },
            "A😀"
        );
        assert_eq!(
            unsafe { owned(tyrus_encoding_utf16_decode_le(le.as_ptr(), le.len())) },
            "A😀"
        );
        assert_eq!(
            unsafe { owned(tyrus_encoding_utf16_decode_be(be.as_ptr(), be.len())) },
            "A😀"
        );
        assert_eq!(
            unsafe { owned(tyrus_encoding_utf16_decode(std::ptr::null(), 0)) },
            ""
        );
    }

    #[test]
    fn strict_utf16_rejects_invalid_surrogates_and_lossy_replaces_them() {
        let invalid = [0xd800u16];
        tyrus_encoding_clear_error();
        assert!(unsafe { tyrus_encoding_utf16_decode(invalid.as_ptr(), invalid.len()) }.is_null());
        let mut error = EncodingErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        };
        unsafe { tyrus_encoding_take_error(&mut error) };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_ENCODING_INVALID_UTF16");
        assert_eq!(
            unsafe { error.message.as_str() },
            "input is not valid UTF-16"
        );
        assert_eq!(
            unsafe {
                owned(tyrus_encoding_utf16_decode_lossy(
                    invalid.as_ptr(),
                    invalid.len(),
                ))
            },
            "�"
        );
        tyrus_encoding_clear_error();
        assert!(unsafe { tyrus_encoding_utf16_decode_le([0xffu8].as_ptr(), 1) }.is_null());
        assert_eq!(take_code(), "ERR_ENCODING_INVALID_UTF16");
    }

    #[test]
    fn strict_utf8_reports_a_stable_code_and_leaves_no_error_on_success() {
        tyrus_encoding_clear_error();
        let invalid = [0xffu8];
        assert!(unsafe { tyrus_encoding_utf8_decode(invalid.as_ptr(), 1) }.is_null());
        assert_eq!(take_code(), "ERR_ENCODING_INVALID_UTF8");
        let valid = "olá".as_bytes();
        assert_eq!(
            unsafe { owned(tyrus_encoding_utf8_decode(valid.as_ptr(), valid.len())) },
            "olá"
        );
        assert_eq!(take_code(), "");
    }

    #[test]
    fn encodes_utf16_without_changing_the_native_utf8_string() {
        let value = crate::strings::alloc_string("A😀");
        let mut units = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { tyrus_encoding_utf16_encode(value.borrow(), &mut units) };
        assert_eq!(
            unsafe { std::slice::from_raw_parts(units.data.cast::<u16>(), units.length) },
            &[0x0041, 0xd83d, 0xde00]
        );
        unsafe { free(units.data.cast()) };
        unsafe { value.release() };
    }

    #[test]
    fn hex_encodes_lowercase_and_decodes_round_trip() {
        let bytes = [0xde, 0xad, 0xbe, 0xef, 0x00, 0xff];
        assert_eq!(
            unsafe { owned(tyrus_encoding_hex_encode(bytes.as_ptr(), bytes.len())) },
            "deadbeef00ff"
        );
        assert_eq!(
            decode_bytes(tyrus_encoding_hex_decode, "deadbeef00ff").unwrap(),
            bytes
        );
        assert_eq!(
            unsafe { owned(tyrus_encoding_hex_encode(ptr::null(), 0)) },
            ""
        );
    }

    #[test]
    fn strict_hex_rejects_invalid_input() {
        for text in ["abc", "0x41", "41 42", "ZZ", "DEad", "4"] {
            assert_eq!(
                decode_bytes(tyrus_encoding_hex_decode, text),
                Err("ERR_ENCODING_INVALID_HEX".to_string()),
                "`{text}`"
            );
        }
    }

    fn decode_bytes(
        decode: unsafe extern "C" fn(TyrusString, *mut TyrusArray),
        text: &str,
    ) -> Result<Vec<u8>, String> {
        tyrus_encoding_clear_error();
        let value = crate::strings::alloc_string(text);
        let mut out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { decode(value.borrow(), &mut out) };
        unsafe { value.release() };
        let code = take_code();
        let bytes = if out.length == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(out.data.cast::<u8>(), out.length) }.to_vec()
        };
        unsafe { free(out.data.cast()) };
        if code.is_empty() {
            Ok(bytes)
        } else {
            Err(code)
        }
    }

    #[test]
    fn base64_encodes_with_padding_and_round_trips() {
        assert_eq!(
            unsafe { owned(tyrus_encoding_base64_encode(ptr::null(), 0)) },
            ""
        );
        let bytes: &[u8] = b"\x00\x01\x02\xde\xad\xbe\xef\xff";
        assert_eq!(
            unsafe { owned(tyrus_encoding_base64_encode(bytes.as_ptr(), bytes.len())) },
            "AAEC3q2+7/8=",
        );
        assert_eq!(
            decode_bytes(tyrus_encoding_base64_decode, "AAEC3q2+7/8=").unwrap(),
            bytes
        );
    }

    #[test]
    fn base64_decoders_reject_the_other_variant_with_the_base64_code() {
        for text in ["Zg", "Zg=c", "-_-_", "Z g=", "AAEC3q2+7z=="] {
            assert_eq!(
                decode_bytes(tyrus_encoding_base64_decode, text),
                Err("ERR_ENCODING_INVALID_BASE64".to_string()),
                "`{text}` must be rejected by standard base64"
            );
        }
        for text in ["Zg==", "+/8", "1", "Z g"] {
            assert_eq!(
                decode_bytes(tyrus_encoding_base64url_decode, text),
                Err("ERR_ENCODING_INVALID_BASE64".to_string()),
                "`{text}` must be rejected by base64url"
            );
        }
        let bytes = [0xfbu8, 0xff, 0x00];
        assert_eq!(
            unsafe { owned(tyrus_encoding_base64url_encode(bytes.as_ptr(), bytes.len())) },
            "-_8A"
        );
    }

    #[test]
    fn base32_round_trips_and_rejects_non_canonical_input() {
        let bytes = b"foobar";
        assert_eq!(
            unsafe { owned(tyrus_encoding_base32_encode(bytes.as_ptr(), bytes.len())) },
            "MZXW6YTBOI======"
        );
        assert_eq!(
            decode_bytes(tyrus_encoding_base32_decode, "MZXW6YTBOI======").unwrap(),
            bytes
        );
        for text in ["my======", "MZXW6YQ", "========", "MZXW7==="] {
            assert_eq!(
                decode_bytes(tyrus_encoding_base32_decode, text),
                Err("ERR_ENCODING_INVALID_BASE32".to_string()),
                "`{text}` must be rejected by base32"
            );
        }
    }

    #[test]
    fn radix_formats_in_each_supported_base() {
        for &(value, base, expected) in &[
            (0i64, 2, "0"),
            (255, 2, "11111111"),
            (255, 8, "377"),
            (255, 10, "255"),
            (-255, 16, "-ff"),
            (i64::MIN, 10, "-9223372036854775808"),
        ] {
            tyrus_encoding_clear_error();
            assert_eq!(
                unsafe { owned(tyrus_encoding_radix_format(value, base)) },
                expected
            );
            assert_eq!(take_code(), "", "format({value}, {base})");
        }
    }

    #[test]
    fn radix_parse_reports_stable_codes_for_rejected_text() {
        let parse_i64 = |text: &str, base: i32| {
            tyrus_encoding_clear_error();
            let value = crate::strings::alloc_string(text);
            let parsed = unsafe { tyrus_encoding_radix_parse_i64(value.borrow(), base) };
            unsafe { value.release() };
            (parsed, take_code())
        };
        assert_eq!(parse_i64("-ff", 16), (-255, String::new()));
        assert_eq!(
            parse_i64("-9223372036854775808", 10),
            (i64::MIN, String::new())
        );
        for (text, base) in [("1g", 10), ("+10", 10), ("", 10), ("FF", 16), ("10", 3)] {
            assert_eq!(
                parse_i64(text, base),
                (0, "ERR_ENCODING_INVALID_RADIX".to_string()),
                "`{text}` base {base}"
            );
        }
        assert_eq!(
            parse_i64("9223372036854775808", 10),
            (0, "ERR_ENCODING_INPUT_TOO_LARGE".to_string())
        );

        let parse_u64 = |text: &str| {
            tyrus_encoding_clear_error();
            let value = crate::strings::alloc_string(text);
            let parsed = unsafe { tyrus_encoding_radix_parse_u64(value.borrow(), 10) };
            unsafe { value.release() };
            (parsed, take_code())
        };
        assert_eq!(parse_u64("18446744073709551615"), (u64::MAX, String::new()));
        assert_eq!(
            parse_u64("18446744073709551616"),
            (0, "ERR_ENCODING_INPUT_TOO_LARGE".to_string())
        );
        assert_eq!(
            parse_u64("-1"),
            (0, "ERR_ENCODING_INVALID_RADIX".to_string())
        );
    }
}
