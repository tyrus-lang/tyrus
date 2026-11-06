//! Native array storage and C ABI operations.

use super::*;
use std::collections::{HashMap, HashSet};
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn realloc(pointer: *mut c_void, size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

unsafe fn drop_string_elements(array: &mut TyrusArray) {
    unsafe {
        if !array.data.is_null() {
            let strings =
                std::slice::from_raw_parts_mut(array.data.cast::<TyrusString>(), array.length);
            for string in strings {
                std::mem::replace(string, TyrusString::null()).release();
            }
            free(array.data.cast());
        }
        array.data = std::ptr::null_mut();
        array.length = 0;
        array.capacity = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_clone_strings(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let (Some(source), Some(output)) = (source.as_ref(), output.as_mut()) else {
            return -1;
        };
        *output = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if validate_array(source, std::mem::size_of::<TyrusString>()).is_err() {
            return -1;
        }
        if source.length == 0 {
            return 0;
        }
        let bytes = match source
            .length
            .checked_mul(std::mem::size_of::<TyrusString>())
        {
            Some(bytes) => bytes,
            None => return -1,
        };
        let data = realloc(std::ptr::null_mut(), bytes).cast::<TyrusString>();
        if data.is_null() {
            return -1;
        }
        let source_strings =
            std::slice::from_raw_parts(source.data.cast::<TyrusString>(), source.length);
        for (index, source_string) in source_strings.iter().enumerate() {
            let cloned = if source_string.is_null() {
                TyrusString::null()
            } else {
                source_string
                    .clone_owned()
                    .unwrap_or_else(TyrusString::null)
            };
            if !source_string.is_null() && cloned.is_null() {
                let initialized = std::slice::from_raw_parts_mut(data, index);
                for string in initialized {
                    std::mem::replace(string, TyrusString::null()).release();
                }
                free(data.cast());
                return -1;
            }
            data.add(index).write(cloned);
        }
        output.data = data.cast();
        output.length = source.length;
        output.capacity = source.length;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_drop_string_contents(array: *mut TyrusArray) {
    unsafe {
        if let Some(array) = array.as_mut() {
            drop_string_elements(array);
        }
    }
}

pub type TyrusArrayCloneElement = Option<unsafe extern "C" fn(*mut c_void, *const c_void)>;
pub type TyrusArrayDropElement = Option<unsafe extern "C" fn(*mut c_void)>;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_clone_owned(
    source: *const TyrusArray,
    output: *mut TyrusArray,
    element_size: u64,
    clone_element: TyrusArrayCloneElement,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(source), Some(output), Some(clone_element)) =
            (source.as_ref(), output.as_mut(), clone_element)
        else {
            return -1;
        };
        *output = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if element_size == 0 || validate_array(source, element_size).is_err() {
            return -1;
        }
        let Some(byte_len) = source.length.checked_mul(element_size) else {
            return -1;
        };
        let data = if byte_len == 0 {
            std::ptr::null_mut()
        } else {
            malloc(byte_len)
        };
        if byte_len != 0 && data.is_null() {
            return -1;
        }
        for index in 0..source.length {
            clone_element(
                data.cast::<u8>().add(index * element_size).cast(),
                source.data.cast::<u8>().add(index * element_size).cast(),
            );
        }
        output.data = data.cast();
        output.length = source.length;
        output.capacity = source.length;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_own_elements(
    array: *mut TyrusArray,
    element_size: u64,
    clone_element: TyrusArrayCloneElement,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array), Some(clone_element)) = (array.as_mut(), clone_element) else {
            return -1;
        };
        if element_size == 0 || validate_array(array, element_size).is_err() {
            return -1;
        }
        for index in 0..array.length {
            let slot = array
                .data
                .cast::<u8>()
                .add(index * element_size)
                .cast::<c_void>();
            clone_element(slot, slot);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_drop_owned_contents(
    array: *mut TyrusArray,
    element_size: u64,
    drop_element: TyrusArrayDropElement,
) {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array), Some(drop_element)) = (array.as_mut(), drop_element) else {
            return;
        };
        if !array.data.is_null() {
            for index in 0..array.length {
                drop_element(array.data.cast::<u8>().add(index * element_size).cast());
            }
            free(array.data.cast());
        }
        array.data = std::ptr::null_mut();
        array.length = 0;
        array.capacity = 0;
    }
}

const STRING_ELEMENT: i32 = 1;
const F32_ELEMENT: i32 = 2;
const F64_ELEMENT: i32 = 3;

unsafe fn validate_array(array: *const TyrusArray, element_size: usize) -> Result<(), ()> {
    unsafe {
        let array = array.as_ref().ok_or(())?;
        let length = array.length;
        let byte_length = length.checked_mul(element_size).ok_or(())?;
        if byte_length > 0 && array.data.is_null() {
            return Err(());
        }
        Ok(())
    }
}

unsafe fn ensure_array_capacity(
    array: &mut TyrusArray,
    required: usize,
    element_size: usize,
) -> Result<(), ()> {
    unsafe {
        let capacity = array.capacity;
        if required <= capacity {
            return Ok(());
        }
        let capacity = capacity.max(4).checked_mul(2).ok_or(())?.max(required);
        let bytes = capacity.checked_mul(element_size).ok_or(())?;
        let data = realloc(array.data.cast(), bytes).cast::<u8>();
        if data.is_null() {
            return Err(());
        }
        array.data = data;
        array.capacity = capacity;
        Ok(())
    }
}

#[cfg(feature = "os-entropy")]
fn random_array_index(upper: usize) -> usize {
    rand::random_range(0..upper)
}

#[cfg(not(feature = "os-entropy"))]
fn random_array_index(upper: usize) -> usize {
    use std::sync::atomic::{AtomicU64, Ordering};

    static STATE: AtomicU64 = AtomicU64::new(0xA409_3822_299F_31D0);
    let upper = upper as u64;
    let threshold = u64::MAX - u64::MAX % upper;
    loop {
        let mut current = STATE.load(Ordering::Relaxed);
        loop {
            let mut next = current;
            next ^= next << 13;
            next ^= next >> 7;
            next ^= next << 17;
            if let Err(observed) =
                STATE.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            {
                current = observed;
                continue;
            }
            if next < threshold {
                return (next % upper) as usize;
            }
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_reserve(
    array: *mut TyrusArray,
    required: i64,
    element_size: i64,
) -> i32 {
    unsafe {
        let (Some(array), Ok(required), Ok(element_size)) = (
            array.as_mut(),
            usize::try_from(required),
            usize::try_from(element_size),
        ) else {
            return -1;
        };
        if element_size == 0 || ensure_array_capacity(array, required, element_size).is_err() {
            return -1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_with_capacity(
    capacity: i64,
    element_size: i64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(output) = output.as_mut() else {
            return -1;
        };
        let Ok(capacity) = usize::try_from(capacity) else {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return -1;
        };
        let Ok(element_size) = usize::try_from(element_size) else {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return -1;
        };
        if element_size == 0 || capacity == 0 {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return 0;
        }
        let bytes = match capacity.checked_mul(element_size) {
            Some(b) => b,
            None => {
                *output = TyrusArray {
                    data: std::ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                return -1;
            }
        };
        let data = malloc(bytes).cast::<u8>();
        if data.is_null() {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return -1;
        }
        *output = TyrusArray {
            data,
            length: 0,
            capacity,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_push(
    array: *mut TyrusArray,
    value: *const u8,
    element_size: u64,
) -> i64 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array), Some(length)) =
            (array.as_mut(), array.as_ref().map(|array| array.length))
        else {
            return -1;
        };
        if value.is_null() || ensure_array_capacity(array, length + 1, element_size).is_err() {
            return -1;
        }
        std::ptr::copy_nonoverlapping(value, array.data.add(length * element_size), element_size);
        array.length += 1;
        array.length as i64
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_unshift(
    array: *mut TyrusArray,
    value: *const u8,
    element_size: u64,
) -> i64 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let length = array.length;
        if value.is_null() || ensure_array_capacity(array, length + 1, element_size).is_err() {
            return -1;
        }
        std::ptr::copy(
            array.data,
            array.data.add(element_size),
            length * element_size,
        );
        std::ptr::copy_nonoverlapping(value, array.data, element_size);
        array.length += 1;
        array.length as i64
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_push_owned_string(
    array: *mut TyrusArray,
    value: TyrusString,
) -> i64 {
    unsafe {
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let Some(required) = array.length.checked_add(1) else {
            return -1;
        };
        if value.is_null()
            || ensure_array_capacity(array, required, std::mem::size_of::<TyrusString>()).is_err()
        {
            return -1;
        }
        let cloned = value.clone_owned().unwrap_or_else(TyrusString::null);
        if cloned.is_null() {
            return -1;
        }
        array
            .data
            .cast::<TyrusString>()
            .add(array.length)
            .write(cloned);
        array.length = required;
        array.length as i64
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_unshift_owned_string(
    array: *mut TyrusArray,
    value: TyrusString,
) -> i64 {
    unsafe {
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let Some(required) = array.length.checked_add(1) else {
            return -1;
        };
        if value.is_null()
            || ensure_array_capacity(array, required, std::mem::size_of::<TyrusString>()).is_err()
        {
            return -1;
        }
        let cloned = value.clone_owned().unwrap_or_else(TyrusString::null);
        if cloned.is_null() {
            return -1;
        }
        let elements = array.data.cast::<TyrusString>();
        std::ptr::copy(elements, elements.add(1), array.length);
        elements.write(cloned);
        array.length = required;
        array.length as i64
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_pop(array: *mut TyrusArray, element_size: u64) -> *mut u8 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return std::ptr::null_mut();
        };
        if array.length == 0 || validate_array(array, element_size).is_err() {
            return std::ptr::null_mut();
        }
        let storage = malloc(element_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return std::ptr::null_mut();
        }
        let last = array.length - 1;
        std::ptr::copy_nonoverlapping(array.data.add(last * element_size), storage, element_size);
        array.length = last;
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_shift(array: *mut TyrusArray, element_size: u64) -> *mut u8 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return std::ptr::null_mut();
        };
        if array.length == 0 || validate_array(array, element_size).is_err() {
            return std::ptr::null_mut();
        }
        let storage = malloc(element_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(array.data, storage, element_size);
        let remaining = array.length - 1;
        std::ptr::copy(
            array.data.add(element_size),
            array.data,
            remaining * element_size,
        );
        array.length = remaining;
        storage
    }
}

fn normalize_index(index: i64, length: usize) -> usize {
    if index < 0 {
        (length as i64 + index).max(0) as usize
    } else {
        (index as usize).min(length)
    }
}

unsafe fn array_values_equal(left: *const u8, right: *const u8, size: usize, kind: i32) -> bool {
    unsafe {
        match kind {
            STRING_ELEMENT => {
                let left = &*(left.cast::<TyrusString>());
                let right = &*(right.cast::<TyrusString>());
                !left.is_null() && !right.is_null() && left.as_bytes() == right.as_bytes()
            }
            F32_ELEMENT => *(left.cast::<f32>()) == *(right.cast::<f32>()),
            F64_ELEMENT => *(left.cast::<f64>()) == *(right.cast::<f64>()),
            _ => std::slice::from_raw_parts(left, size) == std::slice::from_raw_parts(right, size),
        }
    }
}

unsafe fn float_unique_key(element: *const u8, kind: i32) -> Option<u64> {
    unsafe {
        if kind == F32_ELEMENT {
            let value = *(element.cast::<f32>());
            if value.is_nan() {
                return None;
            }
            let value = if value == 0.0 { 0.0 } else { value };
            return Some(u64::from(value.to_bits()));
        }
        let value = *(element.cast::<f64>());
        if value.is_nan() {
            return None;
        }
        let value = if value == 0.0 { 0.0 } else { value };
        Some(value.to_bits())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_equals(
    left: *const TyrusArray,
    right: *const TyrusArray,
    element_size: u64,
    kind: i32,
) -> bool {
    unsafe {
        let element_size = element_size as usize;
        let (Some(left), Some(right)) = (left.as_ref(), right.as_ref()) else {
            return false;
        };
        if validate_array(left, element_size).is_err()
            || validate_array(right, element_size).is_err()
        {
            return false;
        }
        if left.length != right.length {
            return false;
        }
        (0..left.length).all(|index| {
            array_values_equal(
                left.data.add(index * element_size),
                right.data.add(index * element_size),
                element_size,
                kind,
            )
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_index_of(
    array: *const TyrusArray,
    value: *const u8,
    element_size: u64,
    kind: i32,
    reverse: bool,
) -> i64 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_ref() else {
            return -1;
        };
        let length = array.length;
        if value.is_null() || validate_array(array, element_size).is_err() {
            return -1;
        }
        let indices: Box<dyn Iterator<Item = usize>> = if reverse {
            Box::new((0..length).rev())
        } else {
            Box::new(0..length)
        };
        for index in indices {
            if array_values_equal(
                array.data.add(index * element_size),
                value,
                element_size,
                kind,
            ) {
                return index as i64;
            }
        }
        -1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_count_equal(
    array: *const TyrusArray,
    value: *const u8,
    element_size: u64,
    kind: i32,
) -> i64 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_ref() else {
            return -1;
        };
        if value.is_null() || validate_array(array, element_size).is_err() {
            return -1;
        }
        let mut count: i64 = 0;
        for index in 0..array.length {
            if array_values_equal(
                array.data.add(index * element_size),
                value,
                element_size,
                kind,
            ) {
                count += 1;
            }
        }
        count
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_unique(
    array: *const TyrusArray,
    element_size: u64,
    kind: i32,
) -> i64 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_ref() else {
            return 0;
        };
        let length = array.length;
        if validate_array(array, element_size).is_err() {
            return 0;
        }
        if matches!(kind, F32_ELEMENT | F64_ELEMENT) {
            let mut seen: HashSet<u64> = HashSet::with_capacity(length);
            for index in 0..length {
                let Some(key) = float_unique_key(array.data.add(index * element_size), kind) else {
                    continue;
                };
                if !seen.insert(key) {
                    return 0;
                }
            }
            return 1;
        }
        let mut seen: HashSet<&[u8]> = HashSet::with_capacity(length);
        for index in 0..length {
            let element = array.data.add(index * element_size);
            let key: &[u8] = if kind == STRING_ELEMENT {
                let text = &*(element.cast::<TyrusString>());
                if text.is_null() {
                    continue;
                }
                text.as_bytes()
            } else {
                std::slice::from_raw_parts(element, element_size)
            };
            if !seen.insert(key) {
                return 0;
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_unique_values(
    array: *const TyrusArray,
    output: *mut TyrusArray,
    element_size: u64,
    kind: i32,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array), Some(output)) = (array.as_ref(), output.as_mut()) else {
            return -1;
        };
        *output = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if element_size == 0 || validate_array(array, element_size).is_err() {
            return -1;
        }
        let mut seen_bytes: HashSet<Vec<u8>> = HashSet::with_capacity(array.length);
        let mut seen_floats: HashSet<u64> = HashSet::with_capacity(array.length);
        for index in 0..array.length {
            let source = array.data.add(index * element_size);
            let is_new = if matches!(kind, F32_ELEMENT | F64_ELEMENT) {
                let Some(key) = float_unique_key(source, kind) else {
                    continue;
                };
                seen_floats.insert(key)
            } else if kind == STRING_ELEMENT {
                let stored = &*(source.cast::<TyrusString>());
                let key = if stored.is_null() {
                    Vec::new()
                } else {
                    stored.as_str().as_bytes().to_vec()
                };
                seen_bytes.insert(key)
            } else {
                seen_bytes.insert(std::slice::from_raw_parts(source, element_size).to_vec())
            };
            if !is_new {
                continue;
            }
            let status = if kind == STRING_ELEMENT {
                let stored = &*(source.cast::<TyrusString>());
                if stored.is_null() {
                    -1
                } else {
                    tyrus_array_push_owned_string(output, stored.borrow())
                }
            } else {
                tyrus_array_push(output, source, element_size as u64)
            };
            if status < 0 {
                if kind == STRING_ELEMENT {
                    drop_string_elements(output);
                } else if !output.data.is_null() {
                    free(output.data.cast());
                }
                return -1;
            }
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_reverse(array: *mut TyrusArray, element_size: u64) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let length = array.length;
        if validate_array(array, element_size).is_err() {
            return -1;
        }
        let mut temporary = vec![0_u8; element_size];
        for left in 0..length / 2 {
            let right = length - left - 1;
            let left_pointer = array.data.add(left * element_size);
            let right_pointer = array.data.add(right * element_size);
            std::ptr::copy_nonoverlapping(left_pointer, temporary.as_mut_ptr(), element_size);
            std::ptr::copy_nonoverlapping(right_pointer, left_pointer, element_size);
            std::ptr::copy_nonoverlapping(temporary.as_ptr(), right_pointer, element_size);
        }
        0
    }
}

unsafe fn copy_array_range(
    array: *const TyrusArray,
    start: i64,
    end: i64,
    element_size: usize,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let (Some(array), Some(output)) = (array.as_ref(), output.as_mut()) else {
            return -1;
        };
        let length = array.length;
        if validate_array(array, element_size).is_err() {
            return -1;
        }
        let start = normalize_index(start, length);
        let end = normalize_index(end, length).max(start);
        let count = end - start;
        let mut data: *mut u8 = std::ptr::null_mut();
        if count > 0 {
            data = realloc(std::ptr::null_mut(), count * element_size).cast();
            if data.is_null() {
                return -1;
            }
            std::ptr::copy_nonoverlapping(
                array.data.add(start * element_size),
                data,
                count * element_size,
            );
        }
        *output = TyrusArray {
            data,
            length: count,
            capacity: count,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_slice(
    array: *const TyrusArray,
    start: i64,
    end: i64,
    element_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        copy_array_range(array, start, end, element_size, output)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_array_random_index(length: i64) -> i64 {
    let Ok(length) = usize::try_from(length) else {
        return -1;
    };
    if length == 0 {
        return -1;
    }
    random_array_index(length) as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_shuffle(
    source: *const TyrusArray,
    element_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        if source.as_ref().is_none_or(|source| source.length < 2) {
            return -1;
        }
        if element_size == 0 || copy_array_range(source, 0, i64::MAX, element_size, output) != 0 {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            return -1;
        };
        let mut temporary = vec![0_u8; element_size];
        for index in (1..output.length).rev() {
            let selected = random_array_index(index + 1);
            if selected == index {
                continue;
            }
            let left = output.data.cast::<u8>().add(index * element_size);
            let right = output.data.cast::<u8>().add(selected * element_size);
            std::ptr::copy_nonoverlapping(left, temporary.as_mut_ptr(), element_size);
            std::ptr::copy_nonoverlapping(right, left, element_size);
            std::ptr::copy_nonoverlapping(temporary.as_ptr(), right, element_size);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_sample(
    source: *const TyrusArray,
    count: i64,
    repeat: i32,
    element_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(source), Some(output), Ok(count)) =
            (source.as_ref(), output.as_mut(), usize::try_from(count))
        else {
            return -1;
        };
        *output = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if count == 0
            || count > source.length
            || element_size == 0
            || validate_array(source, element_size).is_err()
        {
            return -1;
        }
        let Some(bytes) = count.checked_mul(element_size) else {
            return -1;
        };
        let data = malloc(bytes).cast::<u8>();
        if data.is_null() {
            return -1;
        }
        let mut remapped = HashMap::with_capacity(count.saturating_mul(2));
        for index in 0..count {
            let source_index = if repeat != 0 {
                random_array_index(source.length)
            } else {
                let selected = index + random_array_index(source.length - index);
                let chosen = *remapped.get(&selected).unwrap_or(&selected);
                let replacement = *remapped.get(&index).unwrap_or(&index);
                remapped.insert(selected, replacement);
                chosen
            };
            std::ptr::copy_nonoverlapping(
                source.data.cast::<u8>().add(source_index * element_size),
                data.add(index * element_size),
                element_size,
            );
        }
        output.data = data.cast();
        output.length = count;
        output.capacity = count;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_slice_owned_strings(
    array: *const TyrusArray,
    start: i64,
    end: i64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(source) = array.as_ref() else {
            return -1;
        };
        if validate_array(source, std::mem::size_of::<TyrusString>()).is_err() {
            return -1;
        }
        let start = normalize_index(start, source.length);
        let end = normalize_index(end, source.length).max(start);
        let view = TyrusArray {
            data: if source.data.is_null() {
                source.data
            } else {
                source.data.add(start * std::mem::size_of::<TyrusString>())
            },
            length: end - start,
            capacity: end - start,
        };
        tyrus_array_clone_strings(&view, output)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_concat(
    left: *const TyrusArray,
    right: *const TyrusArray,
    element_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(left), Some(right), Some(output)) =
            (left.as_ref(), right.as_ref(), output.as_mut())
        else {
            return -1;
        };
        let (left_length, right_length) = (left.length, right.length);
        if validate_array(left, element_size).is_err()
            || validate_array(right, element_size).is_err()
        {
            return -1;
        }
        let length = left_length + right_length;
        let data = if length == 0 {
            std::ptr::null_mut()
        } else {
            realloc(std::ptr::null_mut(), length * element_size).cast::<u8>()
        };
        if length > 0 && data.is_null() {
            return -1;
        }
        if left_length > 0 {
            std::ptr::copy_nonoverlapping(left.data, data, left_length * element_size);
        }
        if right_length > 0 {
            std::ptr::copy_nonoverlapping(
                right.data,
                data.add(left_length * element_size),
                right_length * element_size,
            );
        }
        *output = TyrusArray {
            data,
            length,
            capacity: length,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_concat_owned_strings(
    left: *const TyrusArray,
    right: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let (Some(left), Some(right)) = (left.as_ref(), right.as_ref()) else {
            return -1;
        };
        if validate_array(left, std::mem::size_of::<TyrusString>()).is_err()
            || validate_array(right, std::mem::size_of::<TyrusString>()).is_err()
        {
            return -1;
        }
        let length = left.length + right.length;
        let data = if length == 0 {
            std::ptr::null_mut()
        } else {
            realloc(
                std::ptr::null_mut(),
                length * std::mem::size_of::<TyrusString>(),
            )
            .cast::<TyrusString>()
        };
        if length > 0 && data.is_null() {
            return -1;
        }
        let mut written = 0;
        for source in [left, right] {
            let source_strings =
                std::slice::from_raw_parts(source.data.cast::<TyrusString>(), source.length);
            for source_string in source_strings {
                let cloned = if source_string.is_null() {
                    TyrusString::null()
                } else {
                    source_string
                        .clone_owned()
                        .unwrap_or_else(TyrusString::null)
                };
                if !source_string.is_null() && cloned.is_null() {
                    let initialized = std::slice::from_raw_parts_mut(data, written);
                    for string in initialized {
                        if !string.is_null() {
                            std::mem::replace(string, TyrusString::null()).release();
                        }
                    }
                    if !data.is_null() {
                        free(data.cast());
                    }
                    return -1;
                }
                data.add(written).write(cloned);
                written += 1;
            }
        }
        *output = TyrusArray {
            data: data.cast(),
            length,
            capacity: length,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_fill(
    array: *mut TyrusArray,
    value: *const u8,
    start: i64,
    end: i64,
    element_size: u64,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let length = array.length;
        if value.is_null() || validate_array(array, element_size).is_err() {
            return -1;
        }
        for index in normalize_index(start, length)..normalize_index(end, length) {
            std::ptr::copy_nonoverlapping(
                value,
                array.data.add(index * element_size),
                element_size,
            );
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_fill_owned_string(
    array: *mut TyrusArray,
    value: TyrusString,
    start: i64,
    end: i64,
) -> i32 {
    unsafe {
        let Some(array) = array.as_mut() else {
            return -1;
        };
        if value.is_null() || validate_array(array, std::mem::size_of::<TyrusString>()).is_err() {
            return -1;
        }
        let start = normalize_index(start, array.length);
        let end = normalize_index(end, array.length).max(start);
        let mut replacements: Vec<TyrusString> = Vec::with_capacity(end - start);
        for _ in start..end {
            let cloned = value.clone_owned().unwrap_or_else(TyrusString::null);
            if cloned.is_null() {
                for replacement in replacements {
                    replacement.release();
                }
                return -1;
            }
            replacements.push(cloned);
        }
        let elements = array.data.cast::<TyrusString>();
        for (index, replacement) in (start..end).zip(replacements) {
            let previous = elements.add(index).read();
            if !previous.is_null() {
                previous.release();
            }
            elements.add(index).write(replacement);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_copy_within(
    array: *mut TyrusArray,
    target: i64,
    start: i64,
    end: i64,
    element_size: u64,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_mut() else {
            return -1;
        };
        let length = array.length;
        if validate_array(array, element_size).is_err() {
            return -1;
        }
        let target = normalize_index(target, length);
        let start = normalize_index(start, length);
        let count =
            (normalize_index(end, length).saturating_sub(start)).min(length.saturating_sub(target));
        if count > 0 {
            std::ptr::copy(
                array.data.add(start * element_size),
                array.data.add(target * element_size),
                count * element_size,
            );
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_copy_within_owned_strings(
    array: *mut TyrusArray,
    target: i64,
    start: i64,
    end: i64,
) -> i32 {
    unsafe {
        let Some(array) = array.as_mut() else {
            return -1;
        };
        if validate_array(array, std::mem::size_of::<TyrusString>()).is_err() {
            return -1;
        }
        let target = normalize_index(target, array.length);
        let start = normalize_index(start, array.length);
        let count = (normalize_index(end, array.length).saturating_sub(start))
            .min(array.length.saturating_sub(target));
        let elements = array.data.cast::<TyrusString>();
        let mut replacements: Vec<TyrusString> = Vec::with_capacity(count);
        for offset in 0..count {
            let source = elements.add(start + offset).read();
            let cloned = if source.is_null() {
                TyrusString::null()
            } else {
                source.clone_owned().unwrap_or_else(TyrusString::null)
            };
            if !source.is_null() && cloned.is_null() {
                for replacement in replacements {
                    if !replacement.is_null() {
                        replacement.release();
                    }
                }
                return -1;
            }
            replacements.push(cloned);
        }
        for (offset, replacement) in replacements.into_iter().enumerate() {
            let destination = elements.add(target + offset);
            let previous = destination.read();
            if !previous.is_null() {
                previous.release();
            }
            destination.write(replacement);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_splice(
    array: *mut TyrusArray,
    start: i64,
    delete_count: i64,
    inserted: *const u8,
    insert_count: u64,
    element_size: u64,
    removed: *mut TyrusArray,
) -> i32 {
    unsafe {
        let insert_count = insert_count as usize;
        let element_size = element_size as usize;
        let (Some(array), Some(removed)) = (array.as_mut(), removed.as_mut()) else {
            return -1;
        };
        *removed = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if validate_array(array, element_size).is_err() || (insert_count > 0 && inserted.is_null())
        {
            return -1;
        }
        let start = normalize_index(start, array.length);
        let delete_count = usize::try_from(delete_count.max(0))
            .unwrap_or(usize::MAX)
            .min(array.length - start);
        let Some(new_length) = array
            .length
            .checked_sub(delete_count)
            .and_then(|length| length.checked_add(insert_count))
        else {
            return -1;
        };
        let removed_bytes = match delete_count.checked_mul(element_size) {
            Some(bytes) => bytes,
            None => return -1,
        };
        let removed_data = if removed_bytes == 0 {
            std::ptr::null_mut()
        } else {
            malloc(removed_bytes).cast::<u8>()
        };
        if removed_bytes > 0 && removed_data.is_null() {
            return -1;
        }
        if ensure_array_capacity(array, new_length, element_size).is_err() {
            free(removed_data.cast());
            return -1;
        }
        if delete_count > 0 {
            std::ptr::copy_nonoverlapping(
                array.data.add(start * element_size),
                removed_data,
                removed_bytes,
            );
        }
        let tail_start = start + delete_count;
        let tail_count = array.length - tail_start;
        if tail_count > 0 && insert_count != delete_count {
            std::ptr::copy(
                array.data.add(tail_start * element_size),
                array.data.add((start + insert_count) * element_size),
                tail_count * element_size,
            );
        }
        if insert_count > 0 {
            std::ptr::copy_nonoverlapping(
                inserted,
                array.data.add(start * element_size),
                insert_count * element_size,
            );
        }
        array.length = new_length;
        *removed = TyrusArray {
            data: removed_data,
            length: delete_count,
            capacity: delete_count,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_splice_owned_strings(
    array: *mut TyrusArray,
    start: i64,
    delete_count: i64,
    inserted: *const TyrusString,
    insert_count: u64,
    removed: *mut TyrusArray,
) -> i32 {
    unsafe {
        let insert_count = insert_count as usize;
        let (Some(array), Some(removed)) = (array.as_mut(), removed.as_mut()) else {
            return -1;
        };
        *removed = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if validate_array(array, std::mem::size_of::<TyrusString>()).is_err()
            || (insert_count > 0 && inserted.is_null())
        {
            return -1;
        }
        let start = normalize_index(start, array.length);
        let delete_count = usize::try_from(delete_count.max(0))
            .unwrap_or(usize::MAX)
            .min(array.length - start);
        let Some(new_length) = array
            .length
            .checked_sub(delete_count)
            .and_then(|length| length.checked_add(insert_count))
        else {
            return -1;
        };
        let mut replacements: Vec<TyrusString> = Vec::with_capacity(insert_count);
        for index in 0..insert_count {
            let source = inserted.add(index).read();
            if source.is_null() {
                for replacement in replacements {
                    replacement.release();
                }
                return -1;
            }
            let replacement = source.clone_owned().unwrap_or_else(TyrusString::null);
            if replacement.is_null() {
                for replacement in replacements {
                    replacement.release();
                }
                return -1;
            }
            replacements.push(replacement);
        }
        let removed_bytes = match delete_count.checked_mul(std::mem::size_of::<TyrusString>()) {
            Some(bytes) => bytes,
            None => {
                for replacement in replacements {
                    replacement.release();
                }
                return -1;
            }
        };
        let removed_data = if removed_bytes == 0 {
            std::ptr::null_mut()
        } else {
            malloc(removed_bytes).cast::<u8>()
        };
        if removed_bytes > 0 && removed_data.is_null() {
            for replacement in replacements {
                replacement.release();
            }
            return -1;
        }
        if ensure_array_capacity(array, new_length, std::mem::size_of::<TyrusString>()).is_err() {
            for replacement in replacements {
                replacement.release();
            }
            free(removed_data.cast());
            return -1;
        }
        let elements = array.data.cast::<TyrusString>();
        if delete_count > 0 {
            std::ptr::copy_nonoverlapping(
                elements.add(start),
                removed_data.cast::<TyrusString>(),
                delete_count,
            );
        }
        let tail_start = start + delete_count;
        let tail_count = array.length - tail_start;
        if tail_count > 0 && insert_count != delete_count {
            std::ptr::copy(
                elements.add(tail_start),
                elements.add(start + insert_count),
                tail_count,
            );
        }
        for (offset, replacement) in replacements.into_iter().enumerate() {
            elements.add(start + offset).write(replacement);
        }
        array.length = new_length;
        *removed = TyrusArray {
            data: removed_data,
            length: delete_count,
            capacity: delete_count,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_with(
    array: *const TyrusArray,
    index: i64,
    value: *const u8,
    element_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array_ref) = array.as_ref() else {
            return -1;
        };
        let length = array_ref.length;
        let normalized = if index < 0 {
            (length as i64 + index) as usize
        } else {
            index as usize
        };
        if (index < 0 && length as i64 + index < 0)
            || normalized >= length
            || copy_array_range(array, 0, length as i64, element_size, output) != 0
        {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            return -1;
        };
        std::ptr::copy_nonoverlapping(
            value,
            output.data.add(normalized * element_size),
            element_size,
        );
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_with_owned_strings(
    array: *const TyrusArray,
    index: i64,
    value: TyrusString,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(array_ref) = array.as_ref() else {
            return -1;
        };
        let length = array_ref.length;
        let normalized = if index < 0 {
            (length as i64 + index) as usize
        } else {
            index as usize
        };
        if (index < 0 && length as i64 + index < 0)
            || normalized >= length
            || tyrus_array_clone_strings(array, output) != 0
        {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            return -1;
        };
        let cloned_value = if value.is_null() {
            TyrusString::null()
        } else {
            value.clone_owned().unwrap_or_else(TyrusString::null)
        };
        if !value.is_null() && cloned_value.is_null() {
            drop_string_elements(output);
            return -1;
        }
        let elements = output.data.cast::<TyrusString>();
        let previous = elements.add(normalized).read();
        if !previous.is_null() {
            previous.release();
        }
        elements.add(normalized).write(cloned_value);
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_at(
    array: *const TyrusArray,
    index: i64,
    element_size: u64,
) -> *mut u8 {
    unsafe {
        let element_size = element_size as usize;
        let Some(array) = array.as_ref() else {
            return std::ptr::null_mut();
        };
        let length = array.length as i64;
        let resolved = if index < 0 { length + index } else { index };
        if resolved < 0 || resolved >= length || validate_array(array, element_size).is_err() {
            return std::ptr::null_mut();
        }
        let storage = realloc(std::ptr::null_mut(), element_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return std::ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(
            array.data.add(resolved as usize * element_size),
            storage,
            element_size,
        );
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_keys(array: *const TyrusArray, output: *mut TyrusArray) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let length = array.as_ref().map_or(0, |array| array.length);
        let element_size = std::mem::size_of::<usize>();
        let data = if length == 0 {
            std::ptr::null_mut()
        } else {
            realloc(std::ptr::null_mut(), length * element_size).cast::<u8>()
        };
        if length > 0 && !data.is_null() {
            let slots = data.cast::<usize>();
            for index in 0..length {
                *slots.add(index) = index;
            }
        }
        *output = TyrusArray {
            data,
            length,
            capacity: length,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_join(
    array: *const TyrusArray,
    separator: TyrusString,
) -> TyrusString {
    unsafe {
        let Some(array) = array.as_ref() else {
            return TyrusString::null();
        };
        let length = array.length;
        let Ok(separator) = text(&separator) else {
            return TyrusString::null();
        };
        if validate_array(array, std::mem::size_of::<TyrusString>()).is_err() {
            return TyrusString::null();
        }
        if length == 0 {
            return crate::strings::alloc_string("");
        }
        let values = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), length);
        let mut joined = String::new();
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                joined.push_str(&separator);
            }
            let Ok(value) = text(value) else {
                return TyrusString::null();
            };
            joined.push_str(&value);
        }
        crate::strings::alloc_string(&joined)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_toggle(
    array: *const TyrusArray,
    item: *const u8,
    element_size: u64,
    kind: i32,
    key_offset: u64,
    key_size: u64,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array_ref), Some(output)) = (array.as_ref(), output.as_mut()) else {
            return -1;
        };
        if item.is_null() || validate_array(array_ref, element_size).is_err() {
            return -1;
        }
        let length = array_ref.length;
        let has_key = key_size > 0;
        let key_offset = key_offset as usize;
        let key_size = key_size as usize;
        let mut found_index: Option<usize> = None;
        if has_key {
            for index in 0..length {
                let elem = array_ref.data.add(index * element_size);
                let left = elem.add(key_offset);
                let right = item.add(key_offset);
                if std::slice::from_raw_parts(left, key_size)
                    == std::slice::from_raw_parts(right, key_size)
                {
                    found_index = Some(index);
                    break;
                }
            }
        } else {
            for index in 0..length {
                if array_values_equal(
                    array_ref.data.add(index * element_size),
                    item,
                    element_size,
                    kind,
                ) {
                    found_index = Some(index);
                    break;
                }
            }
        }
        let new_length = if found_index.is_some() {
            length - 1
        } else {
            length + 1
        };
        let data = if new_length == 0 {
            std::ptr::null_mut()
        } else {
            realloc(std::ptr::null_mut(), new_length * element_size).cast::<u8>()
        };
        if new_length > 0 && data.is_null() {
            return -1;
        }
        if let Some(found) = found_index {
            if found > 0 {
                std::ptr::copy_nonoverlapping(array_ref.data, data, found * element_size);
            }
            if found < length - 1 {
                std::ptr::copy_nonoverlapping(
                    array_ref.data.add((found + 1) * element_size),
                    data.add(found * element_size),
                    (length - found - 1) * element_size,
                );
            }
        } else {
            if length > 0 {
                std::ptr::copy_nonoverlapping(array_ref.data, data, length * element_size);
            }
            std::ptr::copy_nonoverlapping(item, data.add(length * element_size), element_size);
        }
        *output = TyrusArray {
            data,
            length: new_length,
            capacity: new_length,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_array_toggle_all(
    array: *const TyrusArray,
    items: *const TyrusArray,
    element_size: u64,
    kind: i32,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let element_size = element_size as usize;
        let (Some(array_ref), Some(items_ref), Some(output)) =
            (array.as_ref(), items.as_ref(), output.as_mut())
        else {
            return -1;
        };
        if validate_array(array_ref, element_size).is_err()
            || validate_array(items_ref, element_size).is_err()
        {
            return -1;
        }
        let array_len = array_ref.length;
        let items_len = items_ref.length;
        let total = array_len + items_len;
        let data = if total == 0 {
            std::ptr::null_mut()
        } else {
            realloc(std::ptr::null_mut(), total * element_size).cast::<u8>()
        };
        if total > 0 && data.is_null() {
            return -1;
        }
        if array_len > 0 {
            std::ptr::copy_nonoverlapping(array_ref.data, data, array_len * element_size);
        }
        let mut current_len = array_len;
        for item_index in 0..items_len {
            let item = items_ref.data.add(item_index * element_size);
            let mut found_index: Option<usize> = None;
            for index in 0..current_len {
                if array_values_equal(data.add(index * element_size), item, element_size, kind) {
                    found_index = Some(index);
                    break;
                }
            }
            if let Some(found) = found_index {
                if found < current_len - 1 {
                    std::ptr::copy(
                        data.add((found + 1) * element_size),
                        data.add(found * element_size),
                        (current_len - found - 1) * element_size,
                    );
                }
                current_len -= 1;
            } else {
                std::ptr::copy_nonoverlapping(
                    item,
                    data.add(current_len * element_size),
                    element_size,
                );
                current_len += 1;
            }
        }
        *output = TyrusArray {
            data,
            length: current_len,
            capacity: current_len,
        };
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_array_helpers_preserve_membership_and_validate_sample_bounds() {
        let values = [1_i32, 2, 3, 4];
        let input = TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        let mut shuffled = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut sampled = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut invalid = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(
                tyrus_array_shuffle(&input, std::mem::size_of::<i32>() as u64, &mut shuffled),
                0
            );
            let shuffled_values =
                std::slice::from_raw_parts(shuffled.data.cast::<i32>(), shuffled.length);
            let mut sorted = shuffled_values.to_vec();
            sorted.sort_unstable();
            assert_eq!(sorted, values);
            assert_eq!(
                tyrus_array_sample(
                    &input,
                    3,
                    0,
                    std::mem::size_of::<i32>() as u64,
                    &mut sampled
                ),
                0
            );
            let sampled_values =
                std::slice::from_raw_parts(sampled.data.cast::<i32>(), sampled.length);
            assert_eq!(sampled_values.len(), 3);
            assert!(sampled_values.iter().all(|value| values.contains(value)));
            assert_eq!(
                tyrus_array_sample(
                    &input,
                    0,
                    0,
                    std::mem::size_of::<i32>() as u64,
                    &mut invalid
                ),
                -1
            );
            assert_eq!(
                tyrus_array_sample(
                    &input,
                    5,
                    1,
                    std::mem::size_of::<i32>() as u64,
                    &mut invalid
                ),
                -1
            );
            free(shuffled.data.cast());
            free(sampled.data.cast());
        }
    }

    #[test]
    fn unique_values_preserves_first_occurrences_and_order() {
        let values = [3_i32, 1, 3, 2, 1];
        let input = TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        let mut output = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(
                tyrus_array_unique_values(
                    &input,
                    &mut output,
                    std::mem::size_of::<i32>() as u64,
                    0
                ),
                0
            );
            let result = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(result, &[3, 1, 2]);
            free(output.data.cast());
        }
    }

    #[test]
    fn string_elements_are_compared_by_their_text_and_not_their_address() {
        let first = TyrusString::from_str_owned("ada").expect("within the limit");
        let second = TyrusString::from_str_owned("ada").expect("within the limit");
        let third = TyrusString::from_str_owned("grace").expect("within the limit");
        assert_ne!(
            first.as_raw_ptr(),
            second.as_raw_ptr(),
            "two distinct allocations"
        );

        let duplicate = vec![first.borrow(), second.borrow()];
        let distinct = vec![first.borrow(), third.borrow()];
        let size = std::mem::size_of::<TyrusString>();
        let array = |values: &Vec<TyrusString>| TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        unsafe {
            assert_eq!(
                tyrus_array_unique(&array(&duplicate), size as u64, STRING_ELEMENT),
                0
            );
            assert_eq!(
                tyrus_array_unique(&array(&distinct), size as u64, STRING_ELEMENT),
                1
            );
            first.release();
            second.release();
            third.release();
        }
    }

    #[test]
    fn float_elements_are_compared_as_numbers_and_not_as_bytes() {
        let size = std::mem::size_of::<f64>();
        let array = |values: &Vec<f64>| TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };

        let signed_zeros = vec![0.0_f64, -0.0];
        assert_ne!(0.0_f64.to_bits(), (-0.0_f64).to_bits());
        unsafe {
            assert_eq!(
                tyrus_array_unique(&array(&signed_zeros), size as u64, F64_ELEMENT),
                0,
                "`0.0` and `-0.0` are one number, so the array is not unique"
            );
            assert!(array_values_equal(
                signed_zeros.as_ptr().cast(),
                signed_zeros.as_ptr().add(1).cast(),
                size,
                F64_ELEMENT,
            ));
        }

        let nans = vec![f64::NAN, f64::NAN];
        assert_eq!(f64::NAN.to_bits(), f64::NAN.to_bits());
        unsafe {
            assert_eq!(
                tyrus_array_unique(&array(&nans), size as u64, F64_ELEMENT),
                1,
                "no two `NaN`s are equal, so the array is unique"
            );
            assert!(!array_values_equal(
                nans.as_ptr().cast(),
                nans.as_ptr().add(1).cast(),
                size,
                F64_ELEMENT,
            ));
        }

        unsafe {
            assert_eq!(
                tyrus_array_unique(&array(&vec![1.5, 2.5]), size as u64, F64_ELEMENT),
                1
            );
            assert_eq!(
                tyrus_array_unique(&array(&vec![1.5, 1.5]), size as u64, F64_ELEMENT),
                0
            );
        }
    }

    #[test]
    fn a_large_array_is_checked_in_one_pass() {
        let values: Vec<i64> = (0..100_000).collect();
        let unique = TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        let mut with_duplicate = values.clone();
        with_duplicate.push(0);
        let duplicate = TyrusArray {
            data: with_duplicate.as_ptr() as *mut u8,
            length: with_duplicate.len(),
            capacity: with_duplicate.len(),
        };
        let size = std::mem::size_of::<i64>();
        unsafe {
            assert_eq!(tyrus_array_unique(&unique, size as u64, 0), 1);
            assert_eq!(tyrus_array_unique(&duplicate, size as u64, 0), 0);
        }
    }

    #[test]
    fn validates_unique_numeric_arrays() {
        let unique = [1_i32, 2, 3];
        let duplicate = [1_i32, 2, 1];
        let unique_array = TyrusArray {
            data: unique.as_ptr() as *mut u8,
            length: unique.len(),
            capacity: unique.len(),
        };
        let duplicate_array = TyrusArray {
            data: duplicate.as_ptr() as *mut u8,
            length: duplicate.len(),
            capacity: duplicate.len(),
        };
        unsafe {
            assert_eq!(
                tyrus_array_unique(&unique_array, std::mem::size_of::<i32>() as u64, 0),
                1
            );
            assert_eq!(
                tyrus_array_unique(&duplicate_array, std::mem::size_of::<i32>() as u64, 0),
                0
            );
        }
    }

    #[test]
    fn validates_unique_string_arrays() {
        let alpha = TyrusString::literal("alpha");
        let beta = TyrusString::literal("beta");
        let duplicate_alpha = TyrusString::literal("alpha");
        let unique = [alpha.borrow(), beta.borrow()];
        let duplicate = [alpha.borrow(), duplicate_alpha.borrow()];
        let unique_array = TyrusArray {
            data: unique.as_ptr() as *mut u8,
            length: unique.len(),
            capacity: unique.len(),
        };
        let duplicate_array = TyrusArray {
            data: duplicate.as_ptr() as *mut u8,
            length: duplicate.len(),
            capacity: duplicate.len(),
        };
        unsafe {
            assert_eq!(
                tyrus_array_unique(&unique_array, std::mem::size_of::<TyrusString>() as u64, 1),
                1
            );
            assert_eq!(
                tyrus_array_unique(
                    &duplicate_array,
                    std::mem::size_of::<TyrusString>() as u64,
                    1
                ),
                0
            );
        }
    }

    #[test]
    fn clones_and_drops_string_array_contents_independently() {
        let alpha = TyrusString::literal("alpha");
        let beta = TyrusString::literal("beta");
        let source_values = [alpha.borrow(), beta.borrow()];
        let source = TyrusArray {
            data: source_values.as_ptr() as *mut u8,
            length: source_values.len(),
            capacity: source_values.len(),
        };
        let mut cloned = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_clone_strings(&source, &mut cloned), 0);
            assert_eq!(cloned.length, 2);
            let cloned_values =
                std::slice::from_raw_parts(cloned.data.cast::<TyrusString>(), cloned.length);
            assert_ne!(cloned_values[0].as_raw_ptr(), source_values[0].as_raw_ptr());
            assert_ne!(cloned_values[1].as_raw_ptr(), source_values[1].as_raw_ptr());
            assert_eq!(cloned_values[0].as_bytes(), b"alpha");
            assert_eq!(cloned_values[1].as_bytes(), b"beta");
            tyrus_array_drop_string_contents(&mut cloned);
        }
        assert!(cloned.data.is_null());
        assert_eq!(cloned.length, 0);
        assert_eq!(cloned.capacity, 0);
        unsafe {
            assert_eq!(alpha.as_bytes(), b"alpha");
            assert_eq!(beta.as_bytes(), b"beta");
        }
    }

    #[test]
    fn owned_string_growth_clones_values_and_preserves_order() {
        let first = TyrusString::literal("first");
        let last = TyrusString::literal("last");
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_push_owned_string(&mut array, last.borrow()), 1);
            assert_eq!(
                tyrus_array_unshift_owned_string(&mut array, first.borrow()),
                2
            );
            let values = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            assert_ne!(values[0].as_raw_ptr(), first.as_raw_ptr());
            assert_ne!(values[1].as_raw_ptr(), last.as_raw_ptr());
            assert_eq!(values[0].as_bytes(), b"first");
            assert_eq!(values[1].as_bytes(), b"last");
            tyrus_array_drop_string_contents(&mut array);
        }
        assert!(array.data.is_null());
        assert_eq!(array.length, 0);
    }

    #[test]
    fn pop_and_shift_transfer_elements_into_optional_storage() {
        let first = TyrusString::literal("first");
        let middle = TyrusString::literal("middle");
        let last = TyrusString::literal("last");
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_push_owned_string(&mut array, first.borrow()), 1);
            assert_eq!(
                tyrus_array_push_owned_string(&mut array, middle.borrow()),
                2
            );
            assert_eq!(tyrus_array_push_owned_string(&mut array, last.borrow()), 3);
            let popped = tyrus_array_pop(&mut array, std::mem::size_of::<TyrusString>() as u64);
            let shifted = tyrus_array_shift(&mut array, std::mem::size_of::<TyrusString>() as u64);
            assert!(!popped.is_null());
            assert!(!shifted.is_null());
            let popped_string = &*popped.cast::<TyrusString>();
            let shifted_string = &*shifted.cast::<TyrusString>();
            assert_eq!(popped_string.as_bytes(), b"last");
            assert_eq!(shifted_string.as_bytes(), b"first");
            assert_eq!(array.length, 1);
            let remaining = &*array.data.cast::<TyrusString>();
            assert_eq!(remaining.as_bytes(), b"middle");
            std::ptr::read(popped_string as *const TyrusString).release();
            std::ptr::read(shifted_string as *const TyrusString).release();
            free(popped.cast());
            free(shifted.cast());
            tyrus_array_drop_string_contents(&mut array);
            assert!(
                tyrus_array_pop(&mut array, std::mem::size_of::<TyrusString>() as u64).is_null()
            );
            assert!(tyrus_array_shift(std::ptr::null_mut(), 1).is_null());
        }
    }

    #[test]
    fn reverse_reorders_owned_strings_without_duplicating_pointers() {
        let first = TyrusString::literal("first");
        let last = TyrusString::literal("last");
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_push_owned_string(&mut array, first.borrow()), 1);
            assert_eq!(tyrus_array_push_owned_string(&mut array, last.borrow()), 2);
            let before = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            let first_pointer = before[0].borrow();
            let last_pointer = before[1].borrow();
            assert_eq!(
                tyrus_array_reverse(&mut array, std::mem::size_of::<TyrusString>() as u64),
                0
            );
            let after = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            assert_eq!(
                after
                    .iter()
                    .map(tyrus_string::TyrusString::as_raw_ptr)
                    .collect::<Vec<_>>(),
                vec![last_pointer.as_raw_ptr(), first_pointer.as_raw_ptr()]
            );
            assert_eq!(after[0].as_bytes(), b"last");
            assert_eq!(after[1].as_bytes(), b"first");
            tyrus_array_drop_string_contents(&mut array);
        }
    }

    #[test]
    fn fill_and_copy_within_give_every_owned_string_its_own_reference() {
        let alpha = TyrusString::literal("alpha");
        let beta = TyrusString::literal("beta");
        let fill = TyrusString::literal("fill");
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_push_owned_string(&mut array, alpha.borrow()), 1);
            assert_eq!(tyrus_array_push_owned_string(&mut array, beta.borrow()), 2);
            assert_eq!(tyrus_array_push_owned_string(&mut array, alpha.borrow()), 3);
            assert_eq!(
                tyrus_array_fill_owned_string(&mut array, fill.borrow(), 1, 3),
                0
            );
            let filled = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            assert!(filled[1].is_owned() && filled[2].is_owned());
            assert_eq!(filled[1].as_bytes(), b"fill");
            assert_eq!(filled[2].as_bytes(), b"fill");
            assert_eq!(
                tyrus_array_copy_within_owned_strings(&mut array, 1, 0, 2),
                0
            );
            let copied = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            assert_eq!(copied[0].as_bytes(), b"alpha");
            assert_eq!(copied[1].as_bytes(), b"alpha");
            assert_eq!(copied[2].as_bytes(), b"fill");
            assert!(copied[0].is_owned() && copied[1].is_owned());
            tyrus_array_drop_string_contents(&mut array);
        }
    }

    #[test]
    fn splice_transfers_removed_owned_strings_and_clones_insertions() {
        let first = TyrusString::literal("first");
        let middle = TyrusString::literal("middle");
        let last = TyrusString::literal("last");
        let inserted = TyrusString::literal("inserted");
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut removed = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            assert_eq!(tyrus_array_push_owned_string(&mut array, first.borrow()), 1);
            assert_eq!(
                tyrus_array_push_owned_string(&mut array, middle.borrow()),
                2
            );
            assert_eq!(tyrus_array_push_owned_string(&mut array, last.borrow()), 3);
            let insertion = [inserted.borrow()];
            assert_eq!(
                tyrus_array_splice_owned_strings(
                    &mut array,
                    1,
                    1,
                    insertion.as_ptr(),
                    insertion.len() as u64,
                    &mut removed,
                ),
                0
            );
            let values = std::slice::from_raw_parts(array.data.cast::<TyrusString>(), array.length);
            let removed_values =
                std::slice::from_raw_parts(removed.data.cast::<TyrusString>(), removed.length);
            assert_eq!(values[0].as_bytes(), b"first");
            assert_eq!(values[1].as_bytes(), b"inserted");
            assert_eq!(values[2].as_bytes(), b"last");
            assert_ne!(values[1].as_raw_ptr(), inserted.as_raw_ptr());
            assert_eq!(removed_values[0].as_bytes(), b"middle");
            assert!(
                values
                    .iter()
                    .all(|value| value.as_raw_ptr() != removed_values[0].as_raw_ptr())
            );
            tyrus_array_drop_string_contents(&mut removed);
            tyrus_array_drop_string_contents(&mut array);
        }
    }

    #[test]
    fn splice_grows_and_shrinks_copy_arrays_with_clamped_indices() {
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut removed = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe {
            for value in [1_i32, 2, 3, 4] {
                assert!(
                    tyrus_array_push(
                        &mut array,
                        (&value as *const i32).cast(),
                        std::mem::size_of::<i32>() as u64,
                    ) > 0
                );
            }
            let inserted = [7_i32, 8, 9];
            assert_eq!(
                tyrus_array_splice(
                    &mut array,
                    -3,
                    2,
                    inserted.as_ptr().cast(),
                    inserted.len() as u64,
                    std::mem::size_of::<i32>() as u64,
                    &mut removed,
                ),
                0
            );
            assert_eq!(
                std::slice::from_raw_parts(array.data.cast::<i32>(), array.length),
                &[1, 7, 8, 9, 4]
            );
            assert_eq!(
                std::slice::from_raw_parts(removed.data.cast::<i32>(), removed.length),
                &[2, 3]
            );
            free(removed.data.cast());
            removed = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            assert_eq!(
                tyrus_array_splice(
                    &mut array,
                    1,
                    i64::MAX,
                    std::ptr::null(),
                    0,
                    std::mem::size_of::<i32>() as u64,
                    &mut removed,
                ),
                0
            );
            assert_eq!(
                std::slice::from_raw_parts(array.data.cast::<i32>(), array.length),
                &[1]
            );
            assert_eq!(removed.length, 4);
            free(removed.data.cast());
            free(array.data.cast());
        }
    }

    #[test]
    fn count_equal_counts_every_matching_element_not_just_the_first() {
        let size = std::mem::size_of::<i32>();
        let array = |values: &Vec<i32>| TyrusArray {
            data: values.as_ptr() as *mut u8,
            length: values.len(),
            capacity: values.len(),
        };
        let values = vec![1, 3, 3, 4, 3, 6];
        let target = 3;
        let missing = 9;
        unsafe {
            assert_eq!(
                tyrus_array_count_equal(
                    &array(&values),
                    (&target as *const i32).cast(),
                    size as u64,
                    0
                ),
                3
            );
            assert_eq!(
                tyrus_array_count_equal(
                    &array(&values),
                    (&missing as *const i32).cast(),
                    size as u64,
                    0
                ),
                0
            );
        }
    }

    #[test]
    fn count_equal_on_an_empty_array_is_zero() {
        let empty: Vec<i32> = Vec::new();
        let array = TyrusArray {
            data: empty.as_ptr() as *mut u8,
            length: 0,
            capacity: 0,
        };
        let target = 1;
        let size = std::mem::size_of::<i32>();
        unsafe {
            assert_eq!(
                tyrus_array_count_equal(&array, (&target as *const i32).cast(), size as u64, 0),
                0
            );
        }
    }

    #[test]
    fn count_equal_compares_strings_by_text_and_floats_as_numbers() {
        let first = TyrusString::literal("ada");
        let second = TyrusString::literal("ada");
        let third = TyrusString::literal("grace");
        let strings = [first.borrow(), second.borrow(), third.borrow()];
        let string_size = std::mem::size_of::<TyrusString>();
        let string_array = TyrusArray {
            data: strings.as_ptr() as *mut u8,
            length: strings.len(),
            capacity: strings.len(),
        };
        let needle = TyrusString::literal("ada");
        let needle_ptr = needle.borrow();
        let floats = [0.0_f64, -0.0, 1.5];
        let float_size = std::mem::size_of::<f64>();
        let float_array = TyrusArray {
            data: floats.as_ptr() as *mut u8,
            length: floats.len(),
            capacity: floats.len(),
        };
        let zero = 0.0_f64;
        unsafe {
            assert_eq!(
                tyrus_array_count_equal(
                    &string_array,
                    (&needle_ptr as *const TyrusString).cast(),
                    string_size as u64,
                    STRING_ELEMENT
                ),
                2,
                "both `ada` pointers hold equal text, at different addresses"
            );
            assert_eq!(
                tyrus_array_count_equal(
                    &float_array,
                    (&zero as *const f64).cast(),
                    float_size as u64,
                    F64_ELEMENT
                ),
                2,
                "`0.0` and `-0.0` are the same number"
            );
        }
    }

    #[test]
    fn toggle_adds_element_not_present() {
        unsafe {
            let values: [i32; 3] = [1, 2, 3];
            let array = TyrusArray {
                data: values.as_ptr().cast_mut().cast(),
                length: 3,
                capacity: 3,
            };
            let new_value: i32 = 4;
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<i32>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&new_value as *const i32).cast(),
                    element_size,
                    0,
                    0,
                    0,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 4);
            let result = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(result, &[1, 2, 3, 4]);
            free(output.data.cast());
        }
    }

    #[test]
    fn toggle_removes_element_present() {
        unsafe {
            let values: [i32; 3] = [1, 2, 3];
            let array = TyrusArray {
                data: values.as_ptr().cast_mut().cast(),
                length: 3,
                capacity: 3,
            };
            let target: i32 = 2;
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<i32>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&target as *const i32).cast(),
                    element_size,
                    0,
                    0,
                    0,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 2);
            let result = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(result, &[1, 3]);
            free(output.data.cast());
        }
    }

    #[test]
    fn toggle_preserves_order() {
        unsafe {
            let values: [i32; 4] = [10, 20, 30, 40];
            let array = TyrusArray {
                data: values.as_ptr().cast_mut().cast(),
                length: 4,
                capacity: 4,
            };
            let target: i32 = 30;
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<i32>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&target as *const i32).cast(),
                    element_size,
                    0,
                    0,
                    0,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 3);
            let result = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(result, &[10, 20, 40]);
            free(output.data.cast());
        }
    }

    #[test]
    fn toggle_empty_array_adds() {
        unsafe {
            let array = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let value: i32 = 42;
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<i32>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&value as *const i32).cast(),
                    element_size,
                    0,
                    0,
                    0,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 1);
            let result = std::slice::from_raw_parts(output.data.cast::<i32>(), output.length);
            assert_eq!(result, &[42]);
            free(output.data.cast());
        }
    }

    #[test]
    fn toggle_with_key_removes_element_present() {
        #[repr(C)]
        struct Item {
            id: i32,
            name: [u8; 4],
        }
        unsafe {
            let items = [
                Item {
                    id: 1,
                    name: *b"aaaa",
                },
                Item {
                    id: 2,
                    name: *b"bbbb",
                },
                Item {
                    id: 3,
                    name: *b"cccc",
                },
            ];
            let array = TyrusArray {
                data: items.as_ptr().cast_mut().cast(),
                length: 3,
                capacity: 3,
            };
            let target = Item {
                id: 2,
                name: *b"XX__",
            };
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<Item>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&target as *const Item).cast(),
                    element_size,
                    0,
                    0,
                    std::mem::size_of::<i32>() as u64,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 2);
            let result = std::slice::from_raw_parts(output.data.cast::<Item>(), output.length);
            assert_eq!(result[0].id, 1);
            assert_eq!(result[1].id, 3);
            free(output.data.cast());
        }
    }

    #[test]
    fn toggle_with_key_adds_element_not_present() {
        #[repr(C)]
        struct Item {
            id: i32,
            name: [u8; 4],
        }
        unsafe {
            let items = [Item {
                id: 1,
                name: *b"aaaa",
            }];
            let array = TyrusArray {
                data: items.as_ptr().cast_mut().cast(),
                length: 1,
                capacity: 1,
            };
            let target = Item {
                id: 9,
                name: *b"zzzz",
            };
            let mut output = TyrusArray {
                data: std::ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            let element_size = std::mem::size_of::<Item>() as u64;
            assert_eq!(
                tyrus_array_toggle(
                    &array,
                    (&target as *const Item).cast(),
                    element_size,
                    0,
                    0,
                    std::mem::size_of::<i32>() as u64,
                    &mut output,
                ),
                0
            );
            assert_eq!(output.length, 2);
            let result = std::slice::from_raw_parts(output.data.cast::<Item>(), output.length);
            assert_eq!(result[0].id, 1);
            assert_eq!(result[1].id, 9);
            free(output.data.cast());
        }
    }
}
