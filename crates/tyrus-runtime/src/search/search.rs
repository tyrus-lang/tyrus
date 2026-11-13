use crate::TyrusArray;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

#[repr(C)]
pub struct TyrusSearchInfo {
    pub values: TyrusArray,
    pub target: i32,
    pub index: *mut usize,
    pub comparisons: usize,
    pub probes: usize,
}

#[repr(C)]
pub struct TyrusInterpolationSearchInfo {
    pub values: TyrusArray,
    pub target: i32,
    pub index: *mut usize,
    pub comparisons: usize,
    pub probes: usize,
    pub estimated_positions: TyrusArray,
}

#[repr(C)]
pub struct TyrusExponentialSearchInfo {
    pub values: TyrusArray,
    pub target: i32,
    pub index: *mut usize,
    pub comparisons: usize,
    pub probes: usize,
    pub range_expansions: usize,
    pub final_low: usize,
    pub final_high: usize,
}

#[repr(C)]
pub struct TyrusSelectionInfo {
    pub values: TyrusArray,
    pub selected: *mut i32,
    pub rank: usize,
    pub comparisons: usize,
    pub swaps: usize,
    pub partitions: usize,
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

unsafe fn write_array<T>(output: *mut TyrusArray, values: Vec<T>) -> i32 {
    unsafe {
        let Some(output) = output.as_mut() else {
            return -1;
        };
        let bytes = values.len().saturating_mul(std::mem::size_of::<T>());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes)
        };
        if bytes != 0 && data.is_null() {
            return -1;
        }
        if !values.is_empty() {
            ptr::copy_nonoverlapping(values.as_ptr(), data.cast::<T>(), values.len());
        }
        *output = TyrusArray {
            data: data.cast(),
            length: values.len(),
            capacity: values.len(),
        };
        0
    }
}

unsafe fn optional_box<T>(value: Option<T>) -> *mut T {
    unsafe {
        let Some(value) = value else {
            return ptr::null_mut();
        };
        let pointer = malloc(std::mem::size_of::<T>()).cast::<T>();
        if !pointer.is_null() {
            pointer.write(value);
        }
        pointer
    }
}

fn binary_search_info(values: &[i32], target: i32) -> (Option<usize>, usize, usize) {
    let (mut low, mut high, mut comparisons, mut probes) = (0, values.len(), 0, 0);
    while low < high {
        let middle = low + (high - low) / 2;
        probes += 1;
        comparisons += 1;
        match values[middle].cmp(&target) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return (Some(middle), comparisons, probes),
        }
    }
    (None, comparisons, probes)
}

fn interpolation_search_info(
    values: &[i32],
    target: i32,
) -> (Option<usize>, usize, usize, Vec<usize>) {
    if values.is_empty() {
        return (None, 0, 0, Vec::new());
    }
    let (mut low, mut high, mut comparisons, mut probes, mut positions) =
        (0usize, values.len() - 1, 0usize, 0usize, Vec::new());
    while low <= high && target >= values[low] && target <= values[high] {
        if values[low] == values[high] {
            comparisons += 1;
            probes += 1;
            return (
                (values[low] == target).then_some(low),
                comparisons,
                probes,
                positions,
            );
        }
        let span = (values[high] as i64 - values[low] as i64) as u64;
        let offset = (target as i64 - values[low] as i64) as u64;
        let position = low + (offset.saturating_mul((high - low) as u64) / span) as usize;
        positions.push(position);
        probes += 1;
        comparisons += 1;
        match values[position].cmp(&target) {
            std::cmp::Ordering::Less => low = position + 1,
            std::cmp::Ordering::Greater => {
                if position == 0 {
                    break;
                }
                high = position - 1;
            }
            std::cmp::Ordering::Equal => return (Some(position), comparisons, probes, positions),
        }
    }
    (None, comparisons, probes, positions)
}

fn exponential_search_info(
    values: &[i32],
    target: i32,
) -> (Option<usize>, usize, usize, usize, usize, usize) {
    if values.is_empty() {
        return (None, 0, 0, 0, 0, 0);
    }
    let mut comparisons = 1;
    if values[0] == target {
        return (Some(0), comparisons, 1, 0, 1, 0);
    }
    let (mut bound, mut expansions) = (1usize, 0usize);
    while bound < values.len() {
        comparisons += 1;
        if values[bound] >= target {
            break;
        }
        bound = bound.saturating_mul(2);
        expansions += 1;
    }
    let low = bound / 2;
    let high = bound.saturating_add(1).min(values.len());
    let (index, binary_comparisons, probes) = binary_search_info(&values[low..high], target);
    (
        index.map(|value| low + value),
        comparisons + binary_comparisons,
        probes + 1,
        expansions,
        low,
        high,
    )
}

fn quickselect_info(values: &mut [i32], k: usize) -> (Option<i32>, usize, usize, usize) {
    if k >= values.len() {
        return (None, 0, 0, 0);
    }
    let (mut left, mut right, mut comparisons, mut swaps, mut partitions) =
        (0, values.len() - 1, 0, 0, 0);
    loop {
        partitions += 1;
        let pivot = values[usize::midpoint(left, right)];
        let (mut i, mut j) = (left, right);
        while i <= j {
            while values[i] < pivot {
                comparisons += 1;
                i += 1;
            }
            comparisons += 1;
            while values[j] > pivot {
                comparisons += 1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }
            comparisons += 1;
            if i <= j {
                values.swap(i, j);
                swaps += 1;
                i += 1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }
        if k <= j {
            right = j;
        } else if k >= i {
            left = i;
        } else {
            return (Some(values[k]), comparisons, swaps, partitions);
        }
    }
}

fn quickselect(values: &mut [i32], k: usize) -> Option<i32> {
    if k >= values.len() {
        return None;
    }
    let (mut left, mut right) = (0, values.len() - 1);
    loop {
        let pivot = values[usize::midpoint(left, right)];
        let (mut i, mut j) = (left, right);
        while i <= j {
            while values[i] < pivot {
                i += 1;
            }
            while values[j] > pivot {
                if j == 0 {
                    break;
                }
                j -= 1;
            }
            if i <= j {
                values.swap(i, j);
                i += 1;
                if j == 0 {
                    break;
                }
                j -= 1;
            }
        }
        if k <= j {
            right = j;
        } else if k >= i {
            left = i;
        } else {
            return Some(values[k]);
        }
    }
}

fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    let (mut low, mut high) = (0, values.len());
    while low < high {
        let middle = low + (high - low) / 2;
        match values[middle].cmp(&target) {
            std::cmp::Ordering::Less => low = middle + 1,
            std::cmp::Ordering::Greater => high = middle,
            std::cmp::Ordering::Equal => return Some(middle),
        }
    }
    None
}

fn interpolation_search(values: &[i32], target: i32) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    let (mut low, mut high) = (0usize, values.len() - 1);
    while low <= high && target >= values[low] && target <= values[high] {
        if values[low] == values[high] {
            return (values[low] == target).then_some(low);
        }
        let span = (values[high] as i64 - values[low] as i64) as u64;
        let offset = (target as i64 - values[low] as i64) as u64;
        let position = low + (offset.saturating_mul((high - low) as u64) / span) as usize;
        match values[position].cmp(&target) {
            std::cmp::Ordering::Less => low = position + 1,
            std::cmp::Ordering::Greater => {
                if position == 0 {
                    break;
                }
                high = position - 1;
            }
            std::cmp::Ordering::Equal => return Some(position),
        }
    }
    None
}

fn exponential_search(values: &[i32], target: i32) -> Option<usize> {
    if values.is_empty() {
        return None;
    }
    if values[0] == target {
        return Some(0);
    }
    let mut bound = 1usize;
    while bound < values.len() && values[bound] < target {
        bound = bound.saturating_mul(2);
    }
    let low = bound / 2;
    let high = bound.saturating_add(1).min(values.len());
    binary_search(&values[low..high], target).map(|index| low + index)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_quickselect(
    source: *const TyrusArray,
    k: usize,
    output: *mut i32,
) -> i32 {
    unsafe {
        let Some(mut values) = read_i32(source) else {
            return 0;
        };
        let Some(value) = quickselect(&mut values, k) else {
            return 0;
        };
        if output.is_null() {
            return 0;
        }
        output.write(value);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_binary_search(
    source: *const TyrusArray,
    target: i32,
    output: *mut usize,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return 0;
        };
        let Some(value) = binary_search(&values, target) else {
            return 0;
        };
        if output.is_null() {
            return 0;
        }
        output.write(value);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_interpolation_search(
    source: *const TyrusArray,
    target: i32,
    output: *mut usize,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return 0;
        };
        let Some(value) = interpolation_search(&values, target) else {
            return 0;
        };
        if output.is_null() {
            return 0;
        }
        output.write(value);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_exponential_search(
    source: *const TyrusArray,
    target: i32,
    output: *mut usize,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return 0;
        };
        let Some(value) = exponential_search(&values, target) else {
            return 0;
        };
        if output.is_null() {
            return 0;
        }
        output.write(value);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_binary_search_info(
    source: *const TyrusArray,
    target: i32,
    output: *mut TyrusSearchInfo,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        let (index, comparisons, probes) = binary_search_info(&values, target);
        let mut snapshot = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_array(&mut snapshot, values) != 0 {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(snapshot.data.cast());
            return -1;
        };
        let index = optional_box(index);
        *output = TyrusSearchInfo {
            values: snapshot,
            target,
            index,
            comparisons,
            probes,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_interpolation_search_info(
    source: *const TyrusArray,
    target: i32,
    output: *mut TyrusInterpolationSearchInfo,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        let (index, comparisons, probes, positions) = interpolation_search_info(&values, target);
        let (mut snapshot, mut estimated_positions) = (
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            },
        );
        if write_array(&mut snapshot, values) != 0
            || write_array(&mut estimated_positions, positions) != 0
        {
            free(snapshot.data.cast());
            free(estimated_positions.data.cast());
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(snapshot.data.cast());
            free(estimated_positions.data.cast());
            return -1;
        };
        *output = TyrusInterpolationSearchInfo {
            values: snapshot,
            target,
            index: optional_box(index),
            comparisons,
            probes,
            estimated_positions,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_exponential_search_info(
    source: *const TyrusArray,
    target: i32,
    output: *mut TyrusExponentialSearchInfo,
) -> i32 {
    unsafe {
        let Some(values) = read_i32(source) else {
            return -1;
        };
        let (index, comparisons, probes, range_expansions, final_low, final_high) =
            exponential_search_info(&values, target);
        let mut snapshot = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_array(&mut snapshot, values) != 0 {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(snapshot.data.cast());
            return -1;
        };
        *output = TyrusExponentialSearchInfo {
            values: snapshot,
            target,
            index: optional_box(index),
            comparisons,
            probes,
            range_expansions,
            final_low,
            final_high,
        };
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_quickselect_info(
    source: *const TyrusArray,
    rank: usize,
    output: *mut TyrusSelectionInfo,
) -> i32 {
    unsafe {
        let Some(mut values) = read_i32(source) else {
            return -1;
        };
        let snapshot_values = values.clone();
        let (selected, comparisons, swaps, partitions) = quickselect_info(&mut values, rank);
        let mut snapshot = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        if write_array(&mut snapshot, snapshot_values) != 0 {
            return -1;
        }
        let Some(output) = output.as_mut() else {
            free(snapshot.data.cast());
            return -1;
        };
        *output = TyrusSelectionInfo {
            values: snapshot,
            selected: optional_box(selected),
            rank,
            comparisons,
            swaps,
            partitions,
        };
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_and_selection_cover_found_missing_and_bounds() {
        let mut values = [7, 1, 4, 9, 2];
        assert_eq!(quickselect(&mut values, 2), Some(4));
        let sorted = [1, 2, 4, 7, 9];
        assert_eq!(binary_search(&sorted, 4), Some(2));
        assert_eq!(binary_search(&sorted, 5), None);
        assert_eq!(interpolation_search(&sorted, 7), Some(3));
        assert_eq!(interpolation_search(&sorted, 6), None);
        assert_eq!(exponential_search(&sorted, 9), Some(4));
        assert_eq!(exponential_search(&sorted, 0), None);
    }
}
