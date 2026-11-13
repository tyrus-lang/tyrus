use crate::TyrusArray;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

/// ABI layout for the immutable `BubbleSortInfo` result exposed by
/// `tyrus:algorithm`. Keep this field order aligned with
/// `builtin_interfaces::bubble_sort_info_fields`.
#[repr(C)]
pub struct TyrusBubbleSortInfo {
    pub original_values: TyrusArray,
    pub sorted_values: TyrusArray,
    pub comparisons: usize,
    pub swaps: usize,
    pub passes: usize,
}

#[repr(C)]
pub struct TyrusSortInfo {
    pub original_values: TyrusArray,
    pub sorted_values: TyrusArray,
    pub comparisons: usize,
    pub swaps: usize,
}

unsafe fn read_i32(source: *const TyrusArray) -> Option<Vec<i32>> {
    unsafe {
        let source = source.as_ref()?;
        if source.length == 0 {
            return Some(Vec::new());
        }
        if source.data.is_null() {
            return None;
        }
        Some(std::slice::from_raw_parts(source.data.cast::<i32>(), source.length).to_vec())
    }
}

unsafe fn write_i32(output: *mut TyrusArray, values: Vec<i32>) -> i32 {
    unsafe {
        let Some(output) = output.as_mut() else {
            return -1;
        };
        let bytes = values.len().saturating_mul(std::mem::size_of::<i32>());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<i32>()
        };
        if bytes != 0 && data.is_null() {
            return -1;
        }
        if !values.is_empty() {
            ptr::copy_nonoverlapping(values.as_ptr(), data, values.len());
        }
        *output = TyrusArray {
            data: data.cast(),
            length: values.len(),
            capacity: values.len(),
        };
        0
    }
}

fn merge_sort(values: &[i32]) -> Vec<i32> {
    if values.len() < 2 {
        return values.to_vec();
    }
    let midpoint = values.len() / 2;
    let left = merge_sort(&values[..midpoint]);
    let right = merge_sort(&values[midpoint..]);
    let mut result = Vec::with_capacity(values.len());
    let (mut i, mut j) = (0, 0);
    while i < left.len() && j < right.len() {
        if left[i] <= right[j] {
            result.push(left[i]);
            i += 1;
        } else {
            result.push(right[j]);
            j += 1;
        }
    }
    result.extend_from_slice(&left[i..]);
    result.extend_from_slice(&right[j..]);
    result
}

fn quick_sort(values: &mut [i32]) {
    if values.len() < 2 {
        return;
    }
    let pivot = values[values.len() / 2];
    let (mut left, mut right) = (0, values.len() - 1);
    while left <= right {
        while values[left] < pivot {
            left += 1;
        }
        while values[right] > pivot {
            if right == 0 {
                break;
            }
            right -= 1;
        }
        if left <= right {
            values.swap(left, right);
            left += 1;
            if right == 0 {
                break;
            }
            right -= 1;
        }
    }
    if right > 0 {
        quick_sort(&mut values[..=right]);
    }
    quick_sort(&mut values[left..]);
}

fn heap_sort(values: &mut [i32]) {
    let length = values.len();
    for start in (0..length / 2).rev() {
        sift_down(values, start, length);
    }
    for end in (1..length).rev() {
        values.swap(0, end);
        sift_down(values, 0, end);
    }
}

fn sift_down(values: &mut [i32], mut root: usize, end: usize) {
    loop {
        let child = root * 2 + 1;
        if child >= end {
            return;
        }
        let mut candidate = child;
        if child + 1 < end && values[child] < values[child + 1] {
            candidate += 1;
        }
        if values[root] >= values[candidate] {
            return;
        }
        values.swap(root, candidate);
        root = candidate;
    }
}

fn counting_sort(values: &[i32], max_value: i32) -> Vec<i32> {
    if max_value < 0 {
        return values.to_vec();
    }
    let mut counts = vec![0usize; max_value as usize + 1];
    for &value in values {
        if value < 0 || value > max_value {
            return values.to_vec();
        }
        counts[value as usize] += 1;
    }
    let mut result = Vec::with_capacity(values.len());
    for (value, count) in counts.into_iter().enumerate() {
        result.extend(std::iter::repeat_n(value as i32, count));
    }
    result
}

fn radix_sort(values: &mut [i32]) {
    let mut buffer = vec![0i32; values.len()];
    for shift in (0..32).step_by(8) {
        let mut counts = [0usize; 256];
        for &value in values.iter() {
            let key = ((value as u32 ^ 0x8000_0000) >> shift & 0xff) as usize;
            counts[key] += 1;
        }
        let mut offset = 0;
        for count in &mut counts {
            let next = offset + *count;
            *count = offset;
            offset = next;
        }
        for &value in values.iter() {
            let key = ((value as u32 ^ 0x8000_0000) >> shift & 0xff) as usize;
            buffer[counts[key]] = value;
            counts[key] += 1;
        }
        values.copy_from_slice(&buffer);
    }
}

fn bucket_sort(values: &[i32]) -> Vec<i32> {
    if values.len() < 2 {
        return values.to_vec();
    }
    let min = *values.iter().min().unwrap();
    let max = *values.iter().max().unwrap();
    if min == max {
        return values.to_vec();
    }
    let mut buckets = vec![Vec::new(); values.len()];
    let range = (max as i64 - min as i64) as usize;
    for &value in values {
        let index = ((value as i64 - min as i64) as usize * (buckets.len() - 1)) / range;
        buckets[index].push(value);
    }
    for bucket in &mut buckets {
        bucket.sort_unstable();
    }
    buckets.into_iter().flatten().collect()
}

fn insertion_sort(values: &mut [i32]) {
    for index in 1..values.len() {
        let mut current = index;
        while current > 0 && values[current] < values[current - 1] {
            values.swap(current, current - 1);
            current -= 1;
        }
    }
}

fn shell_sort(values: &mut [i32]) {
    let mut gap = values.len() / 2;
    while gap > 0 {
        for index in gap..values.len() {
            let value = values[index];
            let mut current = index;
            while current >= gap && values[current - gap] > value {
                values[current] = values[current - gap];
                current -= gap;
            }
            values[current] = value;
        }
        gap /= 2;
    }
}

fn selection_sort(values: &mut [i32]) {
    for index in 0..values.len() {
        let minimum = (index + 1..values.len()).fold(index, |minimum, candidate| {
            if values[candidate] < values[minimum] {
                candidate
            } else {
                minimum
            }
        });
        values.swap(index, minimum);
    }
}

fn bubble_sort(values: &mut [i32]) {
    for end in (1..values.len()).rev() {
        let mut swapped = false;
        for index in 0..end {
            if values[index] > values[index + 1] {
                values.swap(index, index + 1);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
}

fn bubble_sort_info(values: &[i32]) -> (Vec<i32>, usize, usize, usize) {
    let mut sorted = values.to_vec();
    let mut comparisons = 0;
    let mut swaps = 0;
    let mut passes = 0;
    for end in (1..sorted.len()).rev() {
        let mut swapped = false;
        for index in 0..end {
            comparisons += 1;
            if sorted[index] > sorted[index + 1] {
                sorted.swap(index, index + 1);
                swaps += 1;
                swapped = true;
            }
        }
        passes += 1;
        if !swapped {
            break;
        }
    }
    (sorted, comparisons, swaps, passes)
}

fn sort_metrics(values: &[i32], kind: i32) -> (usize, usize) {
    let mut values = values.to_vec();
    let (mut comparisons, mut swaps) = (0, 0);
    match kind {
        0 => {
            fn count(values: &[i32], comparisons: &mut usize) -> Vec<i32> {
                if values.len() < 2 {
                    return values.to_vec();
                }
                let middle = values.len() / 2;
                let left_values = count(&values[..middle], comparisons);
                let right_values = count(&values[middle..], comparisons);
                let (mut left, mut right) = (0, 0);
                let mut merged = Vec::with_capacity(values.len());
                while left < left_values.len() && right < right_values.len() {
                    *comparisons += 1;
                    if left_values[left] <= right_values[right] {
                        merged.push(left_values[left]);
                        left += 1;
                    } else {
                        merged.push(right_values[right]);
                        right += 1;
                    }
                }
                merged.extend_from_slice(&left_values[left..]);
                merged.extend_from_slice(&right_values[right..]);
                merged
            }
            let _ = count(&values, &mut comparisons);
        }
        1 => {
            let mut stack = vec![(0, values.len())];
            while let Some((low, high)) = stack.pop() {
                if high - low < 2 {
                    continue;
                }
                let pivot = values[low + (high - low) / 2];
                let (mut left, mut right) = (low, high - 1);
                while left <= right {
                    while left < high {
                        comparisons += 1;
                        if values[left] >= pivot {
                            break;
                        }
                        left += 1;
                    }
                    while right >= low {
                        comparisons += 1;
                        if values[right] <= pivot {
                            break;
                        }
                        if right == 0 {
                            break;
                        }
                        right -= 1;
                    }
                    if left <= right {
                        if left != right {
                            swaps += 1;
                        }
                        values.swap(left, right);
                        left += 1;
                        if right == 0 {
                            break;
                        }
                        right -= 1;
                    }
                }
                if right > low {
                    stack.push((low, right + 1));
                }
                if left < high {
                    stack.push((left, high));
                }
            }
        }
        2 => {
            fn sift(
                values: &mut [i32],
                mut root: usize,
                end: usize,
                comparisons: &mut usize,
                swaps: &mut usize,
            ) {
                loop {
                    let child = root * 2 + 1;
                    if child >= end {
                        return;
                    }
                    let mut candidate = child;
                    if child + 1 < end {
                        *comparisons += 1;
                        if values[child] < values[child + 1] {
                            candidate += 1;
                        }
                    }
                    *comparisons += 1;
                    if values[root] >= values[candidate] {
                        return;
                    }
                    *swaps += 1;
                    values.swap(root, candidate);
                    root = candidate;
                }
            }
            let len = values.len();
            for start in (0..len / 2).rev() {
                sift(&mut values, start, len, &mut comparisons, &mut swaps);
            }
            for end in (1..len).rev() {
                swaps += 1;
                values.swap(0, end);
                sift(&mut values, 0, end, &mut comparisons, &mut swaps);
            }
        }
        3 | 4 => {}
        5 => {
            if values.len() > 1 {
                let min = *values.iter().min().unwrap();
                let max = *values.iter().max().unwrap();
                if min != max {
                    let mut buckets = vec![Vec::new(); values.len()];
                    let range = (max as i64 - min as i64) as usize;
                    for value in values {
                        let index =
                            ((value as i64 - min as i64) as usize * (buckets.len() - 1)) / range;
                        buckets[index].push(value);
                    }
                    for bucket in &mut buckets {
                        bucket.sort_unstable_by(|left, right| {
                            comparisons += 1;
                            left.cmp(right)
                        });
                    }
                }
            }
        }
        6 => {
            values.sort_by(|left, right| {
                comparisons += 1;
                left.cmp(right)
            });
        }
        7 => {
            for index in 1..values.len() {
                let mut current = index;
                while current > 0 {
                    comparisons += 1;
                    if values[current] >= values[current - 1] {
                        break;
                    }
                    swaps += 1;
                    values.swap(current, current - 1);
                    current -= 1;
                }
            }
        }
        8 => {
            let mut gap = values.len() / 2;
            while gap > 0 {
                for index in gap..values.len() {
                    let value = values[index];
                    let mut current = index;
                    while current >= gap {
                        comparisons += 1;
                        if values[current - gap] <= value {
                            break;
                        }
                        values[current] = values[current - gap];
                        current -= gap;
                    }
                    values[current] = value;
                }
                gap /= 2;
            }
        }
        9 => {
            for index in 0..values.len() {
                let mut min = index;
                for candidate in index + 1..values.len() {
                    comparisons += 1;
                    if values[candidate] < values[min] {
                        min = candidate;
                    }
                }
                if min != index {
                    swaps += 1;
                    values.swap(index, min);
                }
            }
        }
        10 if values.len() > 1 => {
            let (mut start, mut end) = (0, values.len() - 1);
            while start < end {
                let mut changed = false;
                for index in start..end {
                    comparisons += 1;
                    if values[index] > values[index + 1] {
                        swaps += 1;
                        values.swap(index, index + 1);
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
                end -= 1;
                changed = false;
                for index in (start + 1..=end).rev() {
                    comparisons += 1;
                    if values[index - 1] > values[index] {
                        swaps += 1;
                        values.swap(index - 1, index);
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
                start += 1;
            }
        }
        _ => {}
    }
    (comparisons, swaps)
}

unsafe fn write_sort_info(
    source: *const TyrusArray,
    kind: i32,
    max_value: i32,
    output: *mut TyrusSortInfo,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        let (comparisons, swaps) = sort_metrics(&values, kind);
        let mut sorted = values.clone();
        match kind {
            0 => sorted = merge_sort(&values),
            1 => quick_sort(&mut sorted),
            2 => heap_sort(&mut sorted),
            3 => sorted = counting_sort(&values, max_value),
            4 => radix_sort(&mut sorted),
            5 => sorted = bucket_sort(&values),
            6 => sorted.sort(),
            7 => insertion_sort(&mut sorted),
            8 => shell_sort(&mut sorted),
            9 => selection_sort(&mut sorted),
            10 => shaker_sort(&mut sorted),
            _ => return -1,
        }
        let mut original_values = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_i32(&mut original_values, values) != 0 {
            return -1;
        }
        let mut sorted_values = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_i32(&mut sorted_values, sorted) != 0 {
            free(original_values.data.cast());
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(original_values.data.cast());
            free(sorted_values.data.cast());
            return -1;
        };
        *output = TyrusSortInfo {
            original_values,
            sorted_values,
            comparisons,
            swaps,
        };
        0
    }
}

macro_rules! sort_info_export {
    ($name:ident, $kind:expr) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(
            source: *const TyrusArray,
            max_value: i32,
            output: *mut TyrusSortInfo,
        ) -> i32 {
            unsafe { write_sort_info(source, $kind, max_value, output) }
        }
    };
}

sort_info_export!(tyrus_algorithm_merge_sort_info, 0);
sort_info_export!(tyrus_algorithm_quick_sort_info, 1);
sort_info_export!(tyrus_algorithm_heap_sort_info, 2);
sort_info_export!(tyrus_algorithm_counting_sort_info, 3);
sort_info_export!(tyrus_algorithm_radix_sort_info, 4);
sort_info_export!(tyrus_algorithm_bucket_sort_info, 5);
sort_info_export!(tyrus_algorithm_tim_sort_info, 6);
sort_info_export!(tyrus_algorithm_insertion_sort_info, 7);
sort_info_export!(tyrus_algorithm_shell_sort_info, 8);
sort_info_export!(tyrus_algorithm_selection_sort_info, 9);
sort_info_export!(tyrus_algorithm_shaker_sort_info, 10);

fn shaker_sort(values: &mut [i32]) {
    if values.len() < 2 {
        return;
    }
    let (mut start, mut end) = (0, values.len() - 1);
    while start < end {
        let mut swapped = false;
        for index in start..end {
            if values[index] > values[index + 1] {
                values.swap(index, index + 1);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
        end -= 1;
        swapped = false;
        for index in (start + 1..=end).rev() {
            if values[index - 1] > values[index] {
                values.swap(index - 1, index);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
        start += 1;
    }
}

unsafe fn sort_with(
    source: *const TyrusArray,
    output: *mut TyrusArray,
    algorithm: fn(&mut [i32]),
) -> i32 {
    unsafe {
        let Some(mut values) = read_i32(source) else {
            return -1;
        };
        algorithm(&mut values);
        write_i32(output, values)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_merge_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        write_i32(output, merge_sort(&values))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_quick_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, quick_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_heap_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, heap_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_radix_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, radix_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bucket_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        write_i32(output, bucket_sort(&values))
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_tim_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, <[i32]>::sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_insertion_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, insertion_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_shell_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, shell_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_selection_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, selection_sort) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bubble_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, bubble_sort) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_bubble_sort_info(
    source: *const TyrusArray,
    output: *mut TyrusBubbleSortInfo,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        let (sorted, comparisons, swaps, passes) = bubble_sort_info(&values);
        let mut original_values = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_i32(&mut original_values, values) != 0 {
            return -1;
        }
        let mut sorted_values = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_i32(&mut sorted_values, sorted) != 0 {
            free(original_values.data.cast());
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(original_values.data.cast());
            free(sorted_values.data.cast());
            return -1;
        };
        *output = TyrusBubbleSortInfo {
            original_values,
            sorted_values,
            comparisons,
            swaps,
            passes,
        };
        0
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_shaker_sort(
    source: *const TyrusArray,
    output: *mut TyrusArray,
) -> i32 {
    unsafe { sort_with(source, output, shaker_sort) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_counting_sort(
    source: *const TyrusArray,
    max_value: i32,
    output: *mut TyrusArray,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        write_i32(output, counting_sort(&values, max_value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sort_produces_ordered_values() {
        let input = [4, -1, 3, 0, 2, 2];
        let expected = [-1, 0, 2, 2, 3, 4];
        assert_eq!(merge_sort(&input), expected);
        for algorithm in [
            quick_sort,
            heap_sort,
            radix_sort,
            insertion_sort,
            shell_sort,
            selection_sort,
            bubble_sort,
            shaker_sort,
        ] {
            let mut values = input;
            algorithm(&mut values);
            assert_eq!(values, expected);
        }
        assert_eq!(bucket_sort(&input), expected);
        assert_eq!(counting_sort(&[4, 1, 3, 0, 2, 2], 4), [0, 1, 2, 2, 3, 4]);
    }

    #[test]
    fn bubble_sort_info_counts_semantic_operations() {
        let input = [4, 1, 3, 2];
        let (sorted, comparisons, swaps, passes) = bubble_sort_info(&input);
        assert_eq!(input, [4, 1, 3, 2]);
        assert_eq!(sorted, [1, 2, 3, 4]);
        assert_eq!(comparisons, 6);
        assert_eq!(swaps, 4);
        assert_eq!(passes, 3);
    }

    #[test]
    fn bubble_sort_info_stops_after_a_sorted_pass() {
        assert_eq!(bubble_sort_info(&[]), (Vec::new(), 0, 0, 0));
        assert_eq!(bubble_sort_info(&[1]), (vec![1], 0, 0, 0));
        assert_eq!(bubble_sort_info(&[1, 2, 3]), (vec![1, 2, 3], 2, 0, 1));
    }
}
