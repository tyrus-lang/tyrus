use super::*;
use serde_json::{Number, Value};
use tyrus_string::TyrusString;

const JSON_OK: i32 = 0;
const JSON_MISSING: i32 = 1;
const JSON_TYPE_ERROR: i32 = 2;
const JSON_INVALID: i32 = 3;
const JSON_NUMBER_OUT_OF_RANGE: i32 = 4;
const JSON_LIMIT_EXCEEDED: i32 = 5;
const MAX_JSON_INPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_JSON_ARRAY_LENGTH: usize = 1_000_000;
const MAX_JSON_DEPTH: usize = 128;

fn json_depth(value: &Value) -> usize {
    match value {
        Value::Array(values) => 1 + values.iter().map(json_depth).max().unwrap_or(0),
        Value::Object(fields) => 1 + fields.values().map(json_depth).max().unwrap_or(0),
        _ => 0,
    }
}

fn parse_object(body: TyrusString) -> Result<serde_json::Map<String, Value>, i32> {
    let Ok(body) = text(&body) else {
        return Err(JSON_INVALID);
    };
    if body.len() > MAX_JSON_INPUT_BYTES {
        return Err(JSON_LIMIT_EXCEEDED);
    }
    match serde_json::from_str::<Value>(&body) {
        Ok(value) if json_depth(&value) > MAX_JSON_DEPTH => Err(JSON_LIMIT_EXCEEDED),
        Ok(Value::Object(object)) => Ok(object),
        Ok(_) => Err(JSON_TYPE_ERROR),
        Err(_) => Err(JSON_INVALID),
    }
}

fn field_value(body: TyrusString, key: TyrusString) -> Result<Option<Value>, i32> {
    let Ok(key) = text(&key) else {
        return Err(JSON_INVALID);
    };
    let object = parse_object(body)?;
    Ok(object.get(&key).cloned())
}

#[repr(C)]
pub struct TyrusJsonDocument {
    object: serde_json::Map<String, Value>,
    text_sourced: bool,
}

#[repr(C)]
pub struct TyrusJsonArray {
    values: Vec<Value>,
    text_sourced: bool,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_parse(
    body: TyrusString,
    status: *mut i32,
) -> *mut TyrusJsonDocument {
    unsafe { parse_document(body, status, false) }
}

/// Reports whether a runtime-generated JSON scalar is the `null` literal.
///
/// # Safety
///
/// `value` must be a valid borrowed Tyrus string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_is_null_literal(value: TyrusString) -> i32 {
    if unsafe { value.as_str() } == "null" {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_parse_text(
    body: TyrusString,
    status: *mut i32,
) -> *mut TyrusJsonDocument {
    unsafe { parse_document(body, status, true) }
}

unsafe fn parse_document(
    body: TyrusString,
    status: *mut i32,
    text_sourced: bool,
) -> *mut TyrusJsonDocument {
    unsafe {
        match parse_object(body) {
            Ok(object) => {
                if !status.is_null() {
                    *status = JSON_OK;
                }
                Box::into_raw(Box::new(TyrusJsonDocument {
                    object,
                    text_sourced,
                }))
            }
            Err(code) => {
                if !status.is_null() {
                    *status = code;
                }
                ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_free(document: *mut TyrusJsonDocument) {
    unsafe {
        if !document.is_null() {
            drop(Box::from_raw(document));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_array(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    status: *mut i32,
) -> *mut TyrusJsonArray {
    unsafe {
        let result = match document_field_value(document, key) {
            Ok(Some((text_sourced, Value::Array(values))))
                if values.len() <= MAX_JSON_ARRAY_LENGTH =>
            {
                Ok(TyrusJsonArray {
                    values: values.clone(),
                    text_sourced,
                })
            }
            Ok(Some((_, Value::Array(_)))) => Err(JSON_LIMIT_EXCEEDED),
            Ok(Some(_)) => Err(JSON_TYPE_ERROR),
            Ok(None) => Err(JSON_MISSING),
            Err(code) => Err(code),
        };
        match result {
            Ok(array) => {
                if !status.is_null() {
                    *status = JSON_OK;
                }
                Box::into_raw(Box::new(array))
            }
            Err(code) => {
                if !status.is_null() {
                    *status = code;
                }
                ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_raw(
    document: *const TyrusJsonDocument,
    key: TyrusString,
) -> TyrusString {
    unsafe {
        match document_field_value(document, key)
            .ok()
            .flatten()
            .and_then(|(_, value)| serde_json::to_string(value).ok())
        {
            Some(value) => crate::strings::alloc_string(&value),
            None => TyrusString::null(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_variant_tag(
    body: TyrusString,
    key: TyrusString,
) -> TyrusString {
    let Ok(key) = text(&key) else {
        return TyrusString::null();
    };
    let Ok(object) = parse_object(body) else {
        return TyrusString::null();
    };
    let Some(value) = object.get(&key) else {
        return TyrusString::null();
    };
    let rendered = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        _ => return TyrusString::null(),
    };
    crate::strings::alloc_string(&rendered)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_parse(
    body: TyrusString,
    status: *mut i32,
) -> *mut TyrusJsonArray {
    unsafe {
        let result = text(&body)
            .map_err(|()| JSON_INVALID)
            .and_then(|body| {
                if body.len() > MAX_JSON_INPUT_BYTES {
                    return Err(JSON_LIMIT_EXCEEDED);
                }
                serde_json::from_str::<Value>(&body).map_err(|_| JSON_INVALID)
            })
            .and_then(|value| {
                if json_depth(&value) > MAX_JSON_DEPTH {
                    return Err(JSON_LIMIT_EXCEEDED);
                }
                match value {
                    Value::Array(values) if values.len() <= MAX_JSON_ARRAY_LENGTH => {
                        Ok(TyrusJsonArray {
                            values,
                            text_sourced: false,
                        })
                    }
                    Value::Array(_) => Err(JSON_LIMIT_EXCEEDED),
                    _ => Err(JSON_TYPE_ERROR),
                }
            });
        match result {
            Ok(array) => {
                if !status.is_null() {
                    *status = JSON_OK;
                }
                Box::into_raw(Box::new(array))
            }
            Err(code) => {
                if !status.is_null() {
                    *status = code;
                }
                ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_free(array: *mut TyrusJsonArray) {
    unsafe {
        if !array.is_null() {
            drop(Box::from_raw(array));
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_len(array: *const TyrusJsonArray) -> usize {
    unsafe { array.as_ref().map_or(0, |array| array.values.len()) }
}

unsafe fn array_value<'a>(
    array: *const TyrusJsonArray,
    index: usize,
) -> Result<(bool, &'a Value), i32> {
    unsafe {
        let Some(array) = array.as_ref() else {
            return Err(JSON_INVALID);
        };
        array
            .values
            .get(index)
            .map(|value| (array.text_sourced, value))
            .ok_or(JSON_MISSING)
    }
}

macro_rules! array_scalar_getter {
    ($name:ident, $ty:ty, $convert:expr_2021) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            array: *const TyrusJsonArray,
            index: usize,
            out: *mut $ty,
        ) -> i32 {
            unsafe {
                match array_value(array, index) {
                    Ok((text_sourced, value)) => ($convert)(text_sourced, value)
                        .map_or_else(|code| code, |value| write_out(out, value)),
                    Err(code) => code,
                }
            }
        }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_get_string(
    array: *const TyrusJsonArray,
    index: usize,
    out: *mut TyrusString,
) -> i32 {
    unsafe {
        match array_value(array, index) {
            Ok((_, Value::String(value))) => {
                let allocated = crate::strings::alloc_string(value);
                if allocated.is_null() {
                    JSON_TYPE_ERROR
                } else {
                    write_out_string(out, allocated)
                }
            }
            Ok(_) => JSON_TYPE_ERROR,
            Err(code) => code,
        }
    }
}

array_scalar_getter!(
    tyrus_json_array_get_bool,
    bool,
    |text_sourced, value: &Value| field_bool(text_sourced, value)
);
array_scalar_getter!(
    tyrus_json_array_get_i64,
    i64,
    |text_sourced, value: &Value| field_number(text_sourced, value)
        .and_then(|number| number.as_i64().ok_or(JSON_NUMBER_OUT_OF_RANGE))
);
array_scalar_getter!(
    tyrus_json_array_get_i32,
    i32,
    |text_sourced, value: &Value| field_number(text_sourced, value).and_then(|number| number
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
        .ok_or(JSON_NUMBER_OUT_OF_RANGE))
);
array_scalar_getter!(
    tyrus_json_array_get_u64,
    u64,
    |text_sourced, value: &Value| field_number(text_sourced, value)
        .and_then(|number| number.as_u64().ok_or(JSON_NUMBER_OUT_OF_RANGE))
);
array_scalar_getter!(
    tyrus_json_array_get_u32,
    u32,
    |text_sourced, value: &Value| field_number(text_sourced, value).and_then(|number| number
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(JSON_NUMBER_OUT_OF_RANGE))
);
array_scalar_getter!(
    tyrus_json_array_get_f64,
    f64,
    |text_sourced, value: &Value| field_number(text_sourced, value).and_then(|number| number
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or(JSON_NUMBER_OUT_OF_RANGE))
);
array_scalar_getter!(
    tyrus_json_array_get_f32,
    f32,
    |text_sourced, value: &Value| field_number(text_sourced, value).and_then(|number| {
        number
            .as_f64()
            .filter(|value| value.is_finite())
            .map(|value| value as f32)
            .filter(|value| value.is_finite())
            .ok_or(JSON_NUMBER_OUT_OF_RANGE)
    })
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_get_decimal(
    array: *const TyrusJsonArray,
    index: usize,
    out: *mut TyrusDecimal,
) -> i32 {
    unsafe {
        match array_value(array, index) {
            Ok((text_sourced, value)) => field_decimal(text_sourced, value)
                .map_or_else(|code| code, |value| write_out(out, value)),
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_get_raw(
    array: *const TyrusJsonArray,
    index: usize,
) -> TyrusString {
    unsafe {
        match array_value(array, index)
            .ok()
            .and_then(|(_, value)| serde_json::to_string(value).ok())
        {
            Some(value) => crate::strings::alloc_string(&value),
            None => TyrusString::null(),
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_unknown_field(
    document: *const TyrusJsonDocument,
    known_fields: TyrusString,
) -> TyrusString {
    unsafe {
        let Some(document) = document.as_ref() else {
            return TyrusString::null();
        };
        let Ok(known_fields) = text(&known_fields) else {
            return TyrusString::null();
        };
        let known = known_fields
            .split(',')
            .collect::<std::collections::HashSet<_>>();
        document
            .object
            .keys()
            .find(|field| !known.contains(field.as_str()))
            .map_or_else(TyrusString::null, |field| {
                crate::strings::alloc_string(field)
            })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_has_only_fields(
    document: *const TyrusJsonDocument,
    known_fields: TyrusString,
) -> bool {
    unsafe {
        let Some(document) = document.as_ref() else {
            return false;
        };
        let Ok(known_fields) = text(&known_fields) else {
            return false;
        };
        let known = known_fields
            .split(',')
            .filter(|field| !field.is_empty())
            .collect::<std::collections::HashSet<_>>();
        document
            .object
            .keys()
            .all(|field| known.contains(field.as_str()))
    }
}

unsafe fn document_field_value<'a>(
    document: *const TyrusJsonDocument,
    key: TyrusString,
) -> Result<Option<(bool, &'a Value)>, i32> {
    unsafe {
        // SAFETY: the generated caller keeps the document and key alive for this lookup; the returned
        // reference is consumed before the document can be freed.
        let Some(document) = document.as_ref() else {
            return Err(JSON_INVALID);
        };
        let Ok(key) = text(&key) else {
            return Err(JSON_INVALID);
        };
        Ok(document
            .object
            .get(&key)
            .map(|value| (document.text_sourced, value)))
    }
}

fn text_number(text_sourced: bool, value: &Value) -> Option<Result<Number, i32>> {
    if !text_sourced {
        return None;
    }
    let Value::String(value) = value else {
        return None;
    };
    let value = value.trim();
    if let Ok(number) = value.parse::<i64>() {
        return Some(Ok(number.into()));
    }
    if let Ok(number) = value.parse::<u64>() {
        return Some(Ok(number.into()));
    }
    if let Ok(number) = value.parse::<f64>() {
        return Some(
            Number::from_f64(number)
                .filter(|_| number.is_finite())
                .ok_or(JSON_NUMBER_OUT_OF_RANGE),
        );
    }
    Some(Err(JSON_TYPE_ERROR))
}

fn text_bool(text_sourced: bool, value: &Value) -> Option<Result<bool, i32>> {
    if !text_sourced {
        return None;
    }
    let Value::String(value) = value else {
        return None;
    };
    Some(match value.trim() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(JSON_TYPE_ERROR),
    })
}

fn decimal_value(value: &Value) -> Option<TyrusDecimal> {
    TyrusDecimal::parse(&value.as_number()?.to_string())
}

fn field_number(text_sourced: bool, value: &Value) -> Result<Number, i32> {
    match value {
        Value::Number(number) => Ok(number.clone()),
        _ => text_number(text_sourced, value).unwrap_or(Err(JSON_TYPE_ERROR)),
    }
}

fn field_bool(text_sourced: bool, value: &Value) -> Result<bool, i32> {
    match value {
        Value::Bool(value) => Ok(*value),
        _ => text_bool(text_sourced, value).unwrap_or(Err(JSON_TYPE_ERROR)),
    }
}

fn field_decimal(text_sourced: bool, value: &Value) -> Result<TyrusDecimal, i32> {
    if let (true, Value::String(value)) = (text_sourced, value) {
        return TyrusDecimal::parse(value.trim()).ok_or(JSON_TYPE_ERROR);
    }
    decimal_value(value).ok_or(JSON_TYPE_ERROR)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_string(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut TyrusString,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((_, Value::String(value)))) => {
                let allocated = crate::strings::alloc_string(value.as_str());
                if allocated.is_null() {
                    JSON_TYPE_ERROR
                } else {
                    write_out_string(out, allocated)
                }
            }
            Ok(Some(_)) => JSON_TYPE_ERROR,
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_bool(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut bool,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_bool(text_sourced, value)
                .map_or_else(|code| code, |value| write_out(out, value)),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_i64(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut i64,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_i64()
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_i32(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut i32,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_i64()
                        .and_then(|value| i32::try_from(value).ok())
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_u64(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut u64,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_u64()
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_u32(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut u32,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_u64()
                        .and_then(|value| u32::try_from(value).ok())
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_f64(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut f64,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_f32(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut f32,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_number(text_sourced, value).map_or_else(
                |code| code,
                |number| {
                    number
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .map(|value| value as f32)
                        .filter(|value| value.is_finite())
                        .map_or(JSON_NUMBER_OUT_OF_RANGE, |value| write_out(out, value))
                },
            ),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_document_get_decimal(
    document: *const TyrusJsonDocument,
    key: TyrusString,
    out: *mut TyrusDecimal,
) -> i32 {
    unsafe {
        match document_field_value(document, key) {
            Ok(Some((text_sourced, value))) => field_decimal(text_sourced, value)
                .map_or_else(|code| code, |value| write_out(out, value)),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

unsafe fn write_out<T: Copy>(out: *mut T, value: T) -> i32 {
    unsafe {
        if out.is_null() {
            return JSON_INVALID;
        }
        *out = value;
        JSON_OK
    }
}

/// The string sibling of [`write_out`]. A `string` is not `Copy`, two values
/// believing they own one buffer is a double free, so it moves into the slot
/// instead of being bit-copied, and `write` rather than assignment because the
/// slot is uninitialized.
unsafe fn write_out_string(out: *mut TyrusString, value: TyrusString) -> i32 {
    unsafe {
        if out.is_null() {
            value.release();
            return JSON_INVALID;
        }
        std::ptr::write(out, value);
        JSON_OK
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_string(
    body: TyrusString,
    key: TyrusString,
    out: *mut TyrusString,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(Value::String(value))) => {
                let allocated = crate::strings::alloc_string(&value);
                if allocated.is_null() {
                    JSON_NUMBER_OUT_OF_RANGE
                } else {
                    write_out_string(out, allocated)
                }
            }
            Ok(Some(_)) => JSON_TYPE_ERROR,
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_bool(
    body: TyrusString,
    key: TyrusString,
    out: *mut bool,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(Value::Bool(value))) => write_out(out, value),
            Ok(Some(_)) => JSON_TYPE_ERROR,
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_i64(
    body: TyrusString,
    key: TyrusString,
    out: *mut i64,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(value)) => value
                .as_i64()
                .map_or(JSON_TYPE_ERROR, |value| write_out(out, value)),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_u64(
    body: TyrusString,
    key: TyrusString,
    out: *mut u64,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(value)) => value
                .as_u64()
                .map_or(JSON_TYPE_ERROR, |value| write_out(out, value)),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_f64(
    body: TyrusString,
    key: TyrusString,
    out: *mut f64,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(value)) => value
                .as_f64()
                .filter(|value| value.is_finite())
                .map_or(JSON_TYPE_ERROR, |value| write_out(out, value)),
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_get_decimal(
    body: TyrusString,
    key: TyrusString,
    out: *mut TyrusDecimal,
) -> i32 {
    unsafe {
        match field_value(body, key) {
            Ok(Some(value)) => {
                decimal_value(&value).map_or(JSON_TYPE_ERROR, |value| write_out(out, value))
            }
            Ok(None) => JSON_MISSING,
            Err(code) => code,
        }
    }
}

#[repr(C)]
pub struct TyrusJsonBuilder {
    object: serde_json::Map<String, Value>,
}

#[repr(C)]
pub struct TyrusJsonArrayBuilder {
    values: Vec<Value>,
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_json_builder_new() -> *mut TyrusJsonBuilder {
    Box::into_raw(Box::new(TyrusJsonBuilder {
        object: serde_json::Map::new(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_free(builder: *mut TyrusJsonBuilder) {
    unsafe {
        if !builder.is_null() {
            drop(Box::from_raw(builder));
        }
    }
}

unsafe fn builder_and_key<'a>(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
) -> Option<(&'a mut TyrusJsonBuilder, String)> {
    unsafe { Some((builder.as_mut()?, text(&key).ok()?)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_string(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: TyrusString,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        let Ok(value) = text(&value) else {
            return -1;
        };
        builder.object.insert(key, Value::String(value));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_bool(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: bool,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        builder.object.insert(key, Value::Bool(value));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_i64(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: i64,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        builder.object.insert(key, Value::Number(value.into()));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_u64(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: u64,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        builder.object.insert(key, Value::Number(value.into()));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_f64(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: f64,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        let Some(number) = Number::from_f64(value) else {
            return -1;
        };
        builder.object.insert(key, Value::Number(number));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_decimal(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: *const TyrusDecimal,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        let Some(value) = value.as_ref() else {
            return -1;
        };
        let Ok(number) = value.json_number_text().parse::<Number>() else {
            return -1;
        };
        builder.object.insert(key, Value::Number(number));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_add_raw(
    builder: *mut TyrusJsonBuilder,
    key: TyrusString,
    value: TyrusString,
) -> i32 {
    unsafe {
        let Some((builder, key)) = builder_and_key(builder, key) else {
            return -1;
        };
        let Ok(value) = text(&value) else {
            return -1;
        };
        let Ok(value) = serde_json::from_str(&value) else {
            return -1;
        };
        builder.object.insert(key, value);
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_json_array_builder_new() -> *mut TyrusJsonArrayBuilder {
    Box::into_raw(Box::new(TyrusJsonArrayBuilder { values: Vec::new() }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_free(builder: *mut TyrusJsonArrayBuilder) {
    unsafe {
        if !builder.is_null() {
            drop(Box::from_raw(builder));
        }
    }
}

unsafe fn array_builder<'a>(
    builder: *mut TyrusJsonArrayBuilder,
) -> Option<&'a mut TyrusJsonArrayBuilder> {
    unsafe { builder.as_mut() }
}

macro_rules! array_builder_add {
    ($name:ident, $ty:ty, $value:expr_2021) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(builder: *mut TyrusJsonArrayBuilder, value: $ty) -> i32 {
            unsafe {
                let Some(builder) = array_builder(builder) else {
                    return -1;
                };
                builder.values.push($value(value));
                0
            }
        }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_add_string(
    builder: *mut TyrusJsonArrayBuilder,
    value: TyrusString,
) -> i32 {
    unsafe {
        let Some(builder) = array_builder(builder) else {
            return -1;
        };
        let Ok(value) = text(&value) else {
            return -1;
        };
        builder.values.push(Value::String(value));
        0
    }
}
array_builder_add!(tyrus_json_array_builder_add_bool, bool, Value::Bool);
array_builder_add!(tyrus_json_array_builder_add_i64, i64, |value: i64| {
    Value::Number(value.into())
});
array_builder_add!(tyrus_json_array_builder_add_u64, u64, |value: u64| {
    Value::Number(value.into())
});
array_builder_add!(tyrus_json_array_builder_add_f64, f64, |value: f64| {
    Number::from_f64(value).map_or(Value::Null, Value::Number)
});

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_add_decimal(
    builder: *mut TyrusJsonArrayBuilder,
    value: *const TyrusDecimal,
) -> i32 {
    unsafe {
        let Some(builder) = array_builder(builder) else {
            return -1;
        };
        let Some(value) = value.as_ref() else {
            return -1;
        };
        let Ok(number) = value.json_number_text().parse::<Number>() else {
            return -1;
        };
        builder.values.push(Value::Number(number));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_add_raw(
    builder: *mut TyrusJsonArrayBuilder,
    value: TyrusString,
) -> i32 {
    unsafe {
        let Some(builder) = array_builder(builder) else {
            return -1;
        };
        let Ok(value) = text(&value) else {
            return -1;
        };
        let Ok(value) = serde_json::from_str(&value) else {
            return -1;
        };
        builder.values.push(value);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_finish(
    builder: *mut TyrusJsonArrayBuilder,
) -> TyrusString {
    unsafe {
        let Some(builder) = builder.as_ref() else {
            return TyrusString::null();
        };
        serde_json::to_string(&builder.values).map_or_else(
            |_| TyrusString::null(),
            |text| crate::strings::alloc_string(&text),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_array_builder_finish_value(
    builder: *mut TyrusJsonArrayBuilder,
) -> TyrusString {
    unsafe {
        let Some(value) = builder
            .as_ref()
            .and_then(|builder| (builder.values.len() == 1).then(|| &builder.values[0]))
        else {
            return TyrusString::null();
        };
        serde_json::to_string(value).map_or_else(
            |_| TyrusString::null(),
            |text| crate::strings::alloc_string(&text),
        )
    }
}

fn compact_json_with_spaces(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in text.chars() {
        output.push(character);
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '"' {
            in_string = true;
        } else if character == ':' {
            output.push(' ');
        }
    }
    output
}

fn json_text(value: Result<String, serde_json::Error>, pretty: bool) -> TyrusString {
    value
        .ok()
        .map(|text| {
            if pretty {
                let two_spaces = serde_json::to_string_pretty(
                    &serde_json::from_str::<Value>(&text)
                        .expect("JSON emitted by serde_json parses"),
                )
                .expect("JSON value serializes");
                two_spaces
                    .lines()
                    .map(|line| {
                        let indent = line.len() - line.trim_start_matches(' ').len();
                        format!("{}{}", " ".repeat(indent * 2), &line[indent..])
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            } else {
                compact_json_with_spaces(&text)
            }
        })
        .map_or_else(TyrusString::null, |text| {
            crate::strings::alloc_string(&text)
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_finish(builder: *mut TyrusJsonBuilder) -> TyrusString {
    unsafe {
        let Some(builder) = builder.as_ref() else {
            return TyrusString::null();
        };
        json_text(serde_json::to_string(&builder.object), false)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_json_builder_finish_pretty(
    builder: *mut TyrusJsonBuilder,
) -> TyrusString {
    unsafe {
        let Some(builder) = builder.as_ref() else {
            return TyrusString::null();
        };
        json_text(serde_json::to_string(&builder.object), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_flat_json_fields() {
        let body = TyrusString::literal(r#"{"name":"Ada","age":42,"active":true}"#);
        let name_key = TyrusString::literal("name");
        let age_key = TyrusString::literal("age");
        let active_key = TyrusString::literal("active");
        let mut name = TyrusString::null();
        let mut age = 0_i64;
        let mut active = false;

        unsafe {
            assert_eq!(
                tyrus_json_get_string(body.borrow(), name_key.borrow(), &mut name),
                0
            );
            assert_eq!(name.as_str(), "Ada");
            name.release();
            assert_eq!(
                tyrus_json_get_i64(body.borrow(), age_key.borrow(), &mut age),
                0
            );
            assert_eq!(age, 42);
            assert_eq!(
                tyrus_json_get_bool(body.borrow(), active_key.borrow(), &mut active),
                0
            );
            assert!(active);
        }
    }

    #[test]
    fn a_text_channel_reads_each_value_as_the_declared_type() {
        let body = TyrusString::literal(
            r#"{"id": "123", "flag": "true", "limit": "25", "ratio": "1.5", "amount": "9.99"}"#,
        );
        let mut status = 0;
        unsafe {
            let document = tyrus_json_document_parse_text(body.borrow(), &mut status);
            assert_eq!(status, JSON_OK);

            let mut id = TyrusString::null();
            assert_eq!(
                tyrus_json_document_get_string(
                    document,
                    TyrusString::literal("id").borrow(),
                    &mut id
                ),
                JSON_OK
            );
            assert_eq!(text(&id).unwrap(), "123");
            id.release();

            let mut flag = false;
            assert_eq!(
                tyrus_json_document_get_bool(
                    document,
                    TyrusString::literal("flag").borrow(),
                    &mut flag
                ),
                JSON_OK
            );
            assert!(flag);

            let mut limit = 0;
            assert_eq!(
                tyrus_json_document_get_i32(
                    document,
                    TyrusString::literal("limit").borrow(),
                    &mut limit
                ),
                JSON_OK
            );
            assert_eq!(limit, 25);

            let mut ratio = 0.0;
            assert_eq!(
                tyrus_json_document_get_f64(
                    document,
                    TyrusString::literal("ratio").borrow(),
                    &mut ratio
                ),
                JSON_OK
            );
            assert_eq!(ratio, 1.5);

            let mut amount = TyrusDecimal::parse("0").unwrap();
            assert_eq!(
                tyrus_json_document_get_decimal(
                    document,
                    TyrusString::literal("amount").borrow(),
                    &mut amount
                ),
                JSON_OK
            );
            assert_eq!(amount, TyrusDecimal::parse("9.99").unwrap());

            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn a_json_body_never_reads_text_as_a_number() {
        let body = TyrusString::literal(r#"{"limit": "25", "flag": "true"}"#);
        let mut status = 0;
        unsafe {
            let document = tyrus_json_document_parse(body.borrow(), &mut status);
            assert_eq!(status, JSON_OK);

            let mut limit = 0;
            assert_eq!(
                tyrus_json_document_get_i32(
                    document,
                    TyrusString::literal("limit").borrow(),
                    &mut limit
                ),
                JSON_TYPE_ERROR
            );
            let mut flag = false;
            assert_eq!(
                tyrus_json_document_get_bool(
                    document,
                    TyrusString::literal("flag").borrow(),
                    &mut flag
                ),
                JSON_TYPE_ERROR
            );
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn text_that_is_not_the_declared_type_is_rejected() {
        let body = TyrusString::literal(r#"{"limit": "abc", "wide": "99999999999", "flag": "1"}"#);
        let mut status = 0;
        unsafe {
            let document = tyrus_json_document_parse_text(body.borrow(), &mut status);
            assert_eq!(status, JSON_OK);

            let mut limit = 0;
            assert_eq!(
                tyrus_json_document_get_i32(
                    document,
                    TyrusString::literal("limit").borrow(),
                    &mut limit
                ),
                JSON_TYPE_ERROR
            );
            let mut wide = 0;
            assert_eq!(
                tyrus_json_document_get_i32(
                    document,
                    TyrusString::literal("wide").borrow(),
                    &mut wide
                ),
                JSON_NUMBER_OUT_OF_RANGE
            );
            let mut flag = false;
            assert_eq!(
                tyrus_json_document_get_bool(
                    document,
                    TyrusString::literal("flag").borrow(),
                    &mut flag
                ),
                JSON_TYPE_ERROR
            );
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn a_text_channel_array_reads_its_elements_as_the_declared_type() {
        let body = TyrusString::literal(r#"{"tags": ["1", "2"]}"#);
        let mut status = 0;
        unsafe {
            let document = tyrus_json_document_parse_text(body.borrow(), &mut status);
            assert_eq!(status, JSON_OK);
            let array = tyrus_json_document_get_array(
                document,
                TyrusString::literal("tags").borrow(),
                &mut status,
            );
            assert_eq!(status, JSON_OK);

            let mut first = TyrusString::null();
            assert_eq!(tyrus_json_array_get_string(array, 0, &mut first), JSON_OK);
            assert_eq!(text(&first).unwrap(), "1");
            first.release();

            let mut second = 0;
            assert_eq!(tyrus_json_array_get_i32(array, 1, &mut second), JSON_OK);
            assert_eq!(second, 2);

            tyrus_json_array_free(array);
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn builds_flat_json_objects() {
        let id_key = TyrusString::literal("id");
        let active_key = TyrusString::literal("active");
        let id = TyrusString::literal("user-1");
        let builder = tyrus_json_builder_new();

        unsafe {
            assert_eq!(
                tyrus_json_builder_add_string(builder, id_key.borrow(), id.borrow()),
                0
            );
            assert_eq!(
                tyrus_json_builder_add_bool(builder, active_key.borrow(), true),
                0
            );
            let output = tyrus_json_builder_finish(builder);
            let output_text = text(&output).unwrap();
            assert!(output_text.contains("\"id\": \"user-1\""));
            assert!(output_text.contains("\"active\": true"));
            output.release();
            tyrus_json_builder_free(builder);
        }
    }

    #[test]
    fn finishes_an_empty_json_object_as_an_empty_object() {
        let builder = tyrus_json_builder_new();
        unsafe {
            let output = tyrus_json_builder_finish(builder);
            assert_eq!(text(&output).unwrap(), "{}");
            output.release();
            tyrus_json_builder_free(builder);
        }
    }

    #[test]
    fn formats_automatic_json_with_spaced_and_pretty_output() {
        let key = TyrusString::literal("name");
        let value = TyrusString::literal("Ada");
        let builder = tyrus_json_builder_new();
        unsafe {
            assert_eq!(
                tyrus_json_builder_add_string(builder, key.borrow(), value.borrow()),
                0
            );
            let compact = tyrus_json_builder_finish(builder);
            assert_eq!(text(&compact).unwrap(), "{\"name\": \"Ada\"}");
            compact.release();
            let pretty = tyrus_json_builder_finish_pretty(builder);
            assert_eq!(text(&pretty).unwrap(), "{\n    \"name\": \"Ada\"\n}");
            pretty.release();
            tyrus_json_builder_free(builder);
        }
    }

    #[test]
    fn preserves_decimal_json_numbers_without_f64_conversion() {
        let body = r#"{"amount":0.1000000000000000000000000000000001}"#;
        let amount_key = "amount";
        let mut value = TyrusDecimal {
            coefficient: 0,
            exponent: 0,
        };
        let mut status = -1;
        let document =
            unsafe { tyrus_json_document_parse(TyrusString::literal(body), &mut status) };
        assert_eq!(status, JSON_OK);
        let builder = tyrus_json_builder_new();

        unsafe {
            assert_eq!(
                tyrus_json_document_get_decimal(
                    document,
                    TyrusString::literal(amount_key),
                    &mut value
                ),
                JSON_OK
            );
            assert_eq!(
                value.coefficient,
                1_000_000_000_000_000_000_000_000_000_000_001
            );
            assert_eq!(value.exponent, -34);
            assert_eq!(
                tyrus_json_builder_add_decimal(builder, TyrusString::literal(amount_key), &value),
                JSON_OK
            );
            let output = tyrus_json_builder_finish(builder);
            assert_eq!(
                text(&output).unwrap(),
                r#"{"amount": 0.1000000000000000000000000000000001}"#
            );
            output.release();
            tyrus_json_builder_free(builder);
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn parses_a_document_once_and_checks_field_ranges() {
        let body = TyrusString::literal(
            r#"{"name":"Ada","active":true,"attempts":2147483647,"subjectId":18446744073709551615,"ratio":1.5}"#,
        );
        let name_key = TyrusString::literal("name");
        let active_key = TyrusString::literal("active");
        let attempts_key = TyrusString::literal("attempts");
        let subject_id_key = TyrusString::literal("subjectId");
        let ratio_key = TyrusString::literal("ratio");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(body.borrow(), &mut status) };
        assert_eq!(status, JSON_OK);
        assert!(!document.is_null());
        let mut name = TyrusString::null();
        let mut active = false;
        let mut attempts = 0_i32;
        let mut subject_id = 0_u64;
        let mut ratio = 0_f32;
        unsafe {
            assert_eq!(
                tyrus_json_document_get_string(document, name_key.borrow(), &mut name),
                JSON_OK
            );
            assert_eq!(
                tyrus_json_document_get_bool(document, active_key.borrow(), &mut active),
                JSON_OK
            );
            assert_eq!(
                tyrus_json_document_get_i32(document, attempts_key.borrow(), &mut attempts),
                JSON_OK
            );
            assert_eq!(
                tyrus_json_document_get_u64(document, subject_id_key.borrow(), &mut subject_id),
                JSON_OK
            );
            assert_eq!(
                tyrus_json_document_get_f32(document, ratio_key.borrow(), &mut ratio),
                JSON_OK
            );
            assert_eq!(name.as_str(), "Ada");
            assert!(active);
            assert_eq!(attempts, i32::MAX);
            assert_eq!(subject_id, u64::MAX);
            assert_eq!(ratio, 1.5);
            name.release();
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn rejects_invalid_documents_missing_fields_and_numeric_overflow() {
        let invalid = TyrusString::literal("[]");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(invalid.borrow(), &mut status) };
        assert!(document.is_null());
        assert_eq!(status, JSON_TYPE_ERROR);

        let body = TyrusString::literal(r#"{"value":2147483648}"#);
        let value_key = TyrusString::literal("value");
        let missing_key = TyrusString::literal("missing");
        let document = unsafe { tyrus_json_document_parse(body.borrow(), &mut status) };
        let mut value = 0_i32;
        unsafe {
            assert_eq!(
                tyrus_json_document_get_i32(document, value_key.borrow(), &mut value),
                JSON_NUMBER_OUT_OF_RANGE
            );
            assert_eq!(
                tyrus_json_document_get_i32(document, missing_key.borrow(), &mut value),
                JSON_MISSING
            );
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn reports_an_unknown_document_field() {
        let body = TyrusString::literal(r#"{"name":"Ada","extra":true}"#);
        let known = TyrusString::literal("name");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(body, &mut status) };
        assert_eq!(status, JSON_OK);
        let unexpected = unsafe { tyrus_json_document_unknown_field(document, known) };
        unsafe {
            assert_eq!(unexpected.as_str(), "extra");
            unexpected.release();
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn rejects_unknown_fields_at_the_json_object_boundary() {
        let mut status = -1;
        unsafe {
            let valid =
                tyrus_json_document_parse(TyrusString::literal(r#"{"name":"Ada"}"#), &mut status);
            assert_eq!(status, JSON_OK);
            assert!(tyrus_json_document_has_only_fields(
                valid,
                TyrusString::literal("name")
            ));
            tyrus_json_document_free(valid);

            let unexpected = tyrus_json_document_parse(
                TyrusString::literal(r#"{"name":"Ada","admin":true}"#),
                &mut status,
            );
            assert_eq!(status, JSON_OK);
            assert!(!tyrus_json_document_has_only_fields(
                unexpected,
                TyrusString::literal("name")
            ));
            tyrus_json_document_free(unexpected);

            let invalid = tyrus_json_document_parse(TyrusString::literal("[]"), &mut status);
            assert!(invalid.is_null());
            assert!(!tyrus_json_document_has_only_fields(
                invalid,
                TyrusString::literal("name")
            ));
        }
    }

    #[test]
    fn exposes_checked_array_handles() {
        let body = TyrusString::literal(r#"{"values":[1,2,3]}"#);
        let key = TyrusString::literal("values");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(body, &mut status) };
        assert_eq!(status, JSON_OK);
        let array = unsafe { tyrus_json_document_get_array(document, key, &mut status) };
        assert_eq!(status, JSON_OK);
        assert_eq!(unsafe { tyrus_json_array_len(array) }, 3);
        unsafe {
            tyrus_json_array_free(array);
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn reads_and_builds_json_arrays() {
        let body = TyrusString::literal(r#"{"values":[1,2,3]}"#);
        let key = TyrusString::literal("values");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(body, &mut status) };
        let array = unsafe { tyrus_json_document_get_array(document, key, &mut status) };
        let mut second = 0_i32;
        assert_eq!(
            unsafe { tyrus_json_array_get_i32(array, 1, &mut second) },
            JSON_OK
        );
        assert_eq!(second, 2);

        let builder = tyrus_json_array_builder_new();
        unsafe {
            assert_eq!(tyrus_json_array_builder_add_i64(builder, 4), 0);
            assert_eq!(tyrus_json_array_builder_add_i64(builder, 5), 0);
            let output = tyrus_json_array_builder_finish(builder);
            assert_eq!(text(&output).unwrap(), "[4,5]");
            output.release();
            tyrus_json_array_builder_free(builder);
            tyrus_json_array_free(array);
            tyrus_json_document_free(document);
        }
    }

    #[test]
    fn serializes_one_array_builder_value_as_a_json_scalar() {
        let builder = tyrus_json_array_builder_new();
        unsafe {
            assert_eq!(tyrus_json_array_builder_add_i64(builder, 42), 0);
            let output = tyrus_json_array_builder_finish_value(builder);
            assert_eq!(text(&output).unwrap(), "42");
            output.release();
            tyrus_json_array_builder_free(builder);
        }
    }

    #[test]
    fn serializes_an_empty_array_builder_as_an_empty_json_array() {
        let builder = tyrus_json_array_builder_new();
        unsafe {
            let output = tyrus_json_array_builder_finish(builder);
            assert_eq!(text(&output).unwrap(), "[]");
            output.release();
            tyrus_json_array_builder_free(builder);
        }
    }

    #[test]
    fn reports_out_of_range_numbers_in_json_arrays() {
        let body = TyrusString::literal(r#"{"values":[2147483648]}"#);
        let key = TyrusString::literal("values");
        let mut status = -1;
        let document = unsafe { tyrus_json_document_parse(body, &mut status) };
        let array = unsafe { tyrus_json_document_get_array(document, key, &mut status) };
        let mut value = 0_i32;
        assert_eq!(
            unsafe { tyrus_json_array_get_i32(array, 0, &mut value) },
            JSON_NUMBER_OUT_OF_RANGE
        );
        unsafe {
            tyrus_json_array_free(array);
            tyrus_json_document_free(document);
        }
    }
}
