use std::cmp::Ordering;
use std::ffi::{c_char, c_void};
use std::ptr;
use std::slice;
use tyrus_string::TyrusString;

use crate::TyrusArray;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

#[repr(C)]
pub struct TyrusSuffixArrayInfo {
    pub text: TyrusString,
    pub suffix_array: TyrusArray,
    pub sort_round_count: usize,
    pub comparison_count: usize,
    pub compared_byte_count: usize,
}

fn compare_suffixes(
    bytes: &[u8],
    left: usize,
    right: usize,
    comparison_count: &mut usize,
    compared_byte_count: &mut usize,
) -> Ordering {
    *comparison_count += 1;
    let mut offset = 0;
    while left + offset < bytes.len() && right + offset < bytes.len() {
        *compared_byte_count += 1;
        match bytes[left + offset].cmp(&bytes[right + offset]) {
            Ordering::Equal => offset += 1,
            ordering => return ordering,
        }
    }
    (bytes.len() - left).cmp(&(bytes.len() - right))
}

fn build_suffix_array(bytes: &[u8]) -> (Vec<usize>, usize, usize, usize) {
    let mut suffixes: Vec<usize> = (0..bytes.len()).collect();
    let mut scratch = suffixes.clone();
    let mut width = 1;
    let mut rounds = 0;
    let mut comparisons = 0;
    let mut compared_bytes = 0;
    while width < bytes.len() {
        rounds += 1;
        let mut start = 0;
        while start < bytes.len() {
            let middle = (start + width).min(bytes.len());
            let end = (start + width * 2).min(bytes.len());
            let (mut left, mut right, mut output) = (start, middle, start);
            while left < middle && right < end {
                if compare_suffixes(
                    bytes,
                    suffixes[left],
                    suffixes[right],
                    &mut comparisons,
                    &mut compared_bytes,
                ) != Ordering::Greater
                {
                    scratch[output] = suffixes[left];
                    left += 1;
                } else {
                    scratch[output] = suffixes[right];
                    right += 1;
                }
                output += 1;
            }
            while left < middle {
                scratch[output] = suffixes[left];
                left += 1;
                output += 1;
            }
            while right < end {
                scratch[output] = suffixes[right];
                right += 1;
                output += 1;
            }
            start = end;
        }
        std::mem::swap(&mut suffixes, &mut scratch);
        width *= 2;
    }
    (suffixes, rounds, comparisons, compared_bytes)
}

unsafe fn write_suffix_array(output: &mut TyrusArray, suffixes: &[usize]) -> bool {
    unsafe {
        if suffixes.is_empty() {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return true;
        }
        let allocation = malloc(std::mem::size_of_val(suffixes)).cast::<usize>();
        if allocation.is_null() {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return false;
        }
        ptr::copy_nonoverlapping(suffixes.as_ptr(), allocation, suffixes.len());
        *output = TyrusArray {
            data: allocation.cast::<u8>(),
            length: suffixes.len(),
            capacity: suffixes.len(),
        };
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_suffix_array(
    data: *const c_char,
    length: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        if length == 0 || data.is_null() {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        }
        let bytes = slice::from_raw_parts(data.cast::<u8>(), length);
        let (suffixes, _, _, _) = build_suffix_array(bytes);
        write_suffix_array(output, &suffixes);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_suffix_array_info(
    data: *const c_char,
    length: usize,
    output: *mut TyrusSuffixArrayInfo,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        if data.is_null() {
            return;
        }
        let bytes = if length == 0 {
            &[]
        } else {
            slice::from_raw_parts(data.cast::<u8>(), length)
        };
        let (suffixes, sort_round_count, comparison_count, compared_byte_count) =
            build_suffix_array(bytes);
        let mut suffix_array = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if !write_suffix_array(&mut suffix_array, &suffixes) {
            return;
        }
        *output = TyrusSuffixArrayInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(bytes).unwrap_or_default()),
            suffix_array,
            sort_round_count,
            comparison_count,
            compared_byte_count,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::build_suffix_array;

    #[test]
    fn instrumented_suffix_array_has_deterministic_metrics() {
        assert_eq!(
            build_suffix_array(b"banana"),
            (vec![5, 3, 1, 0, 4, 2], 3, 11, 14)
        );
        assert_eq!(build_suffix_array(b""), (vec![], 0, 0, 0));
    }
}
