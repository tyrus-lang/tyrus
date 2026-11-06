use std::collections::{HashMap, VecDeque};
use std::ffi::c_char;
use std::ptr;

#[repr(C)]
pub struct TyrusKmpSearchInfo {
    pub text: TyrusString,
    pub pattern: TyrusString,
    pub matches: TyrusArray,
    pub comparison_count: usize,
    pub shift_count: usize,
    pub prefix_table: TyrusArray,
    pub prefix_fallback_count: usize,
}
#[repr(C)]
pub struct TyrusZAlgorithmInfo {
    pub text: TyrusString,
    pub z_table: TyrusArray,
    pub comparison_count: usize,
    pub window_extension_count: usize,
}
#[repr(C)]
pub struct TyrusRabinKarpInfo {
    pub text: TyrusString,
    pub pattern: TyrusString,
    pub matches: TyrusArray,
    pub comparison_count: usize,
    pub shift_count: usize,
    pub window_count: usize,
    pub hash_collision_count: usize,
    pub text_verification_count: usize,
}
#[repr(C)]
pub struct TyrusBoyerMooreInfo {
    pub text: TyrusString,
    pub pattern: TyrusString,
    pub matches: TyrusArray,
    pub comparison_count: usize,
    pub shift_count: usize,
    pub bad_character_table: TyrusArray,
    pub applied_shifts: TyrusArray,
}
#[repr(C)]
pub struct TyrusAhoCorasickInfo {
    pub text: TyrusString,
    pub patterns: TyrusArray,
    pub matches: TyrusArray,
    pub visited_states: TyrusArray,
    pub transition_count: usize,
    pub failure_follow_count: usize,
}
#[repr(C)]
pub struct TyrusEditDistanceInfo {
    pub left: TyrusString,
    pub right: TyrusString,
    pub distance: usize,
    pub row_count: usize,
    pub column_count: usize,
    pub evaluated_cell_count: usize,
}

use crate::TyrusArray;
use tyrus_string::TyrusString;

unsafe fn bytes(data: *const c_char, length: usize) -> Option<&'static [u8]> {
    unsafe {
        if length == 0 {
            return Some(&[]);
        }
        if data.is_null() {
            return None;
        }
        Some(std::slice::from_raw_parts(data.cast::<u8>(), length))
    }
}

unsafe fn write_usize_array(output: *mut TyrusArray, values: Vec<usize>) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut values = values;
        *output = if values.is_empty() {
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            }
        } else {
            let result = TyrusArray {
                data: values.as_mut_ptr().cast(),
                length: values.len(),
                capacity: values.capacity(),
            };
            std::mem::forget(values);
            result
        };
    }
}

unsafe fn write_pair_array(output: *mut TyrusArray, values: Vec<(usize, usize)>) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut values = values;
        *output = if values.is_empty() {
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            }
        } else {
            let result = TyrusArray {
                data: values.as_mut_ptr().cast(),
                length: values.len(),
                capacity: values.capacity(),
            };
            std::mem::forget(values);
            result
        };
    }
}

unsafe fn clone_string_array(output: *mut TyrusArray, values: &[TyrusString]) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let mut copies = values
            .iter()
            .map(|value| {
                if value.is_null() {
                    TyrusString::null()
                } else {
                    crate::strings::alloc_string(value.as_str())
                }
            })
            .collect::<Vec<TyrusString>>();
        *output = if copies.is_empty() {
            TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            }
        } else {
            let result = TyrusArray {
                data: copies.as_mut_ptr().cast(),
                length: copies.len(),
                capacity: copies.capacity(),
            };
            std::mem::forget(copies);
            result
        };
    }
}

fn kmp_search(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..=text.len()).collect();
    }
    let mut prefix = vec![0; pattern.len()];
    for i in 1..pattern.len() {
        let mut j = prefix[i - 1];
        while j > 0 && pattern[i] != pattern[j] {
            j = prefix[j - 1];
        }
        if pattern[i] == pattern[j] {
            j += 1;
        }
        prefix[i] = j;
    }
    let mut matches = Vec::new();
    let mut matched = 0;
    for (i, &byte) in text.iter().enumerate() {
        while matched > 0 && byte != pattern[matched] {
            matched = prefix[matched - 1];
        }
        if byte == pattern[matched] {
            matched += 1;
        }
        if matched == pattern.len() {
            matches.push(i + 1 - pattern.len());
            matched = prefix[matched - 1];
        }
    }
    matches
}

fn z_algorithm(text: &[u8]) -> Vec<usize> {
    let mut z = vec![0; text.len()];
    let (mut left, mut right) = (0, 0);
    for i in 1..text.len() {
        if i <= right {
            z[i] = (right - i + 1).min(z[i - left]);
        }
        while i + z[i] < text.len() && text[z[i]] == text[i + z[i]] {
            z[i] += 1;
        }
        if i + z[i] > 0 && i + z[i] - 1 > right {
            left = i;
            right = i + z[i] - 1;
        }
    }
    z
}

fn rabin_karp(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..=text.len()).collect();
    }
    if pattern.len() > text.len() {
        return Vec::new();
    }
    const BASE: u64 = 256;
    const MODULUS: u64 = 1_000_000_007;
    let mut high = 1;
    for _ in 1..pattern.len() {
        high = (high * BASE) % MODULUS;
    }
    let mut pattern_hash = 0;
    let mut window_hash = 0;
    for index in 0..pattern.len() {
        pattern_hash = (pattern_hash * BASE + u64::from(pattern[index])) % MODULUS;
        window_hash = (window_hash * BASE + u64::from(text[index])) % MODULUS;
    }
    let mut matches = Vec::new();
    for start in 0..=text.len() - pattern.len() {
        if pattern_hash == window_hash && text[start..start + pattern.len()] == *pattern {
            matches.push(start);
        }
        if start + pattern.len() < text.len() {
            let outgoing = (u64::from(text[start]) * high) % MODULUS;
            window_hash = (window_hash + MODULUS - outgoing) % MODULUS;
            window_hash = (window_hash * BASE + u64::from(text[start + pattern.len()])) % MODULUS;
        }
    }
    matches
}

fn boyer_moore(text: &[u8], pattern: &[u8]) -> Vec<usize> {
    if pattern.is_empty() {
        return (0..=text.len()).collect();
    }
    if pattern.len() > text.len() {
        return Vec::new();
    }
    let mut last = [usize::MAX; 256];
    for (index, &byte) in pattern.iter().enumerate() {
        last[byte as usize] = index;
    }
    let mut matches = Vec::new();
    let mut offset = 0;
    while offset + pattern.len() <= text.len() {
        let mut index = pattern.len();
        while index > 0 && pattern[index - 1] == text[offset + index - 1] {
            index -= 1;
        }
        if index == 0 {
            matches.push(offset);
            offset += 1;
        } else {
            let bad = text[offset + index - 1];
            let shift = match last[bad as usize] {
                usize::MAX => index,
                position if position < index - 1 => index - 1 - position,
                _ => 1,
            };
            offset += shift;
        }
    }
    matches
}

type BoyerMooreInfo = (Vec<usize>, usize, Vec<(usize, usize)>, Vec<usize>);

fn boyer_moore_info(text: &[u8], pattern: &[u8]) -> BoyerMooreInfo {
    let mut last = [usize::MAX; 256];
    for (index, &byte) in pattern.iter().enumerate() {
        last[byte as usize] = index;
    }
    let table = last
        .iter()
        .enumerate()
        .filter_map(|(byte, &index)| (index != usize::MAX).then_some((byte, index)))
        .collect();
    if pattern.is_empty() {
        return ((0..=text.len()).collect(), 0, table, Vec::new());
    }
    if pattern.len() > text.len() {
        return (Vec::new(), 0, table, Vec::new());
    }
    let mut matches = Vec::new();
    let mut comparisons = 0;
    let mut shifts = Vec::new();
    let mut offset = 0;
    while offset + pattern.len() <= text.len() {
        let mut index = pattern.len();
        while index > 0 {
            comparisons += 1;
            if pattern[index - 1] != text[offset + index - 1] {
                break;
            }
            index -= 1;
        }
        let shift = if index == 0 {
            matches.push(offset);
            1
        } else {
            let bad = text[offset + index - 1];
            match last[bad as usize] {
                usize::MAX => index,
                position if position < index - 1 => index - 1 - position,
                _ => 1,
            }
        };
        shifts.push(shift);
        offset += shift;
    }
    (matches, comparisons, table, shifts)
}

fn aho_corasick(text: &[u8], patterns: &[Vec<u8>]) -> Vec<(usize, usize)> {
    let mut transitions = vec![HashMap::new()];
    let mut outputs = vec![Vec::new()];
    let mut lengths = Vec::with_capacity(patterns.len());
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        lengths.push(pattern.len());
        if pattern.is_empty() {
            continue;
        }
        let mut node = 0;
        for &byte in pattern {
            let next = if let Some(&next) = transitions[node].get(&byte) {
                next
            } else {
                let next = transitions.len();
                transitions[node].insert(byte, next);
                transitions.push(HashMap::new());
                outputs.push(Vec::new());
                next
            };
            node = next;
        }
        outputs[node].push(pattern_index);
    }
    let mut failure = vec![0; transitions.len()];
    let mut queue = VecDeque::new();
    for &child in transitions[0].values() {
        queue.push_back(child);
    }
    while let Some(node) = queue.pop_front() {
        let edges = transitions[node].clone();
        for (byte, child) in edges {
            let mut fallback = failure[node];
            while fallback != 0 && !transitions[fallback].contains_key(&byte) {
                fallback = failure[fallback];
            }
            if let Some(&next) = transitions[fallback].get(&byte) {
                failure[child] = next;
            }
            let inherited = outputs[failure[child]].clone();
            outputs[child].extend(inherited);
            queue.push_back(child);
        }
    }
    let mut node = 0;
    let mut matches = Vec::new();
    for (index, &byte) in text.iter().enumerate() {
        while node != 0 && !transitions[node].contains_key(&byte) {
            node = failure[node];
        }
        node = transitions[node].get(&byte).copied().unwrap_or(0);
        for &pattern_index in &outputs[node] {
            if lengths[pattern_index] > 0 {
                matches.push((index, pattern_index));
            }
        }
    }
    matches.sort_unstable();
    matches
}

fn aho_corasick_info(
    text: &[u8],
    patterns: &[Vec<u8>],
) -> (Vec<(usize, usize)>, Vec<usize>, usize, usize) {
    let mut transitions = vec![HashMap::new()];
    let mut outputs = vec![Vec::new()];
    let mut lengths = Vec::with_capacity(patterns.len());
    for (pattern_index, pattern) in patterns.iter().enumerate() {
        lengths.push(pattern.len());
        if pattern.is_empty() {
            continue;
        }
        let mut node = 0;
        for &byte in pattern {
            let next = if let Some(&next) = transitions[node].get(&byte) {
                next
            } else {
                let next = transitions.len();
                transitions[node].insert(byte, next);
                transitions.push(HashMap::new());
                outputs.push(Vec::new());
                next
            };
            node = next;
        }
        outputs[node].push(pattern_index);
    }
    let mut failure = vec![0; transitions.len()];
    let mut queue = VecDeque::new();
    for &child in transitions[0].values() {
        queue.push_back(child);
    }
    while let Some(node) = queue.pop_front() {
        let edges = transitions[node].clone();
        for (byte, child) in edges {
            let mut fallback = failure[node];
            while fallback != 0 && !transitions[fallback].contains_key(&byte) {
                fallback = failure[fallback];
            }
            if let Some(&next) = transitions[fallback].get(&byte) {
                failure[child] = next;
            }
            let inherited = outputs[failure[child]].clone();
            outputs[child].extend(inherited);
            queue.push_back(child);
        }
    }
    let mut node = 0;
    let mut matches = Vec::new();
    let mut visited_states = Vec::with_capacity(text.len());
    let mut failure_follow_count = 0;
    for (index, &byte) in text.iter().enumerate() {
        while node != 0 && !transitions[node].contains_key(&byte) {
            node = failure[node];
            failure_follow_count += 1;
        }
        node = transitions[node].get(&byte).copied().unwrap_or(0);
        visited_states.push(node);
        for &pattern_index in &outputs[node] {
            if lengths[pattern_index] > 0 {
                matches.push((index, pattern_index));
            }
        }
    }
    matches.sort_unstable();
    (matches, visited_states, text.len(), failure_follow_count)
}

fn levenshtein(a: &[u8], b: &[u8]) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, &left) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &right) in b.iter().enumerate() {
            let old = row[j + 1];
            row[j + 1] = if left == right {
                diagonal
            } else {
                1 + row[j].min(old).min(diagonal)
            };
            diagonal = old;
        }
    }
    row[b.len()]
}

fn levenshtein_info(a: &[u8], b: &[u8]) -> (usize, usize) {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    let mut evaluated = 0;
    for (i, &left) in a.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &right) in b.iter().enumerate() {
            let old = row[j + 1];
            evaluated += 1;
            row[j + 1] = if left == right {
                diagonal
            } else {
                1 + row[j].min(old).min(diagonal)
            };
            diagonal = old;
        }
    }
    (row[b.len()], evaluated)
}

unsafe fn string_parts(data: *const c_char, length: usize) -> Option<&'static [u8]> {
    unsafe { bytes(data, length) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kmp_search(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Some(text), Some(pattern)) = (bytes(text, text_len), bytes(pattern, pattern_len))
        else {
            return;
        };
        write_usize_array(output, kmp_search(text, pattern));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_kmp_search_info(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusKmpSearchInfo,
) {
    unsafe {
        let (Some(text_bytes), Some(pattern_bytes), Some(output)) = (
            bytes(text, text_len),
            bytes(pattern, pattern_len),
            output.as_mut(),
        ) else {
            return;
        };
        let mut prefix = vec![0; pattern_bytes.len()];
        let mut comparisons = 0;
        let mut fallbacks = 0;
        for i in 1..pattern_bytes.len() {
            let mut j = prefix[i - 1];
            while j > 0 && pattern_bytes[i] != pattern_bytes[j] {
                comparisons += 1;
                fallbacks += 1;
                j = prefix[j - 1];
            }
            comparisons += 1;
            if pattern_bytes[i] == pattern_bytes[j] {
                j += 1;
            }
            prefix[i] = j;
        }
        let matches = kmp_search(text_bytes, pattern_bytes);
        let mut matches_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut prefix_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        write_usize_array(&mut matches_out, matches);
        write_usize_array(&mut prefix_out, prefix);
        *output = TyrusKmpSearchInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(text_bytes).unwrap_or_default()),
            pattern: crate::strings::alloc_string(
                std::str::from_utf8(pattern_bytes).unwrap_or_default(),
            ),
            matches: matches_out,
            comparison_count: comparisons + text_len,
            shift_count: text_len,
            prefix_table: prefix_out,
            prefix_fallback_count: fallbacks,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_z_algorithm(
    text: *const c_char,
    text_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(text) = bytes(text, text_len) else {
            return;
        };
        write_usize_array(output, z_algorithm(text));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_z_algorithm_info(
    text: *const c_char,
    text_len: usize,
    output: *mut TyrusZAlgorithmInfo,
) {
    unsafe {
        let (Some(text_bytes), Some(output)) = (bytes(text, text_len), output.as_mut()) else {
            return;
        };
        let mut table = vec![0; text_bytes.len()];
        let (mut left, mut right) = (0, 0);
        let mut comparisons = 0;
        let mut extensions = 0;
        for i in 1..text_bytes.len() {
            if i <= right {
                table[i] = (right - i + 1).min(table[i - left]);
            }
            while i + table[i] < text_bytes.len() {
                comparisons += 1;
                if text_bytes[table[i]] != text_bytes[i + table[i]] {
                    break;
                }
                table[i] += 1;
                extensions += 1;
            }
            if i + table[i] > 0 && i + table[i] - 1 > right {
                left = i;
                right = i + table[i] - 1;
            }
        }
        let mut z_table = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        write_usize_array(&mut z_table, table);
        *output = TyrusZAlgorithmInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(text_bytes).unwrap_or_default()),
            z_table,
            comparison_count: comparisons,
            window_extension_count: extensions,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_rabin_karp(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Some(text), Some(pattern)) = (bytes(text, text_len), bytes(pattern, pattern_len))
        else {
            return;
        };
        write_usize_array(output, rabin_karp(text, pattern));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_rabin_karp_info(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusRabinKarpInfo,
) {
    unsafe {
        let (Some(text_bytes), Some(pattern_bytes), Some(output)) = (
            bytes(text, text_len),
            bytes(pattern, pattern_len),
            output.as_mut(),
        ) else {
            return;
        };
        let matches = rabin_karp(text_bytes, pattern_bytes);
        let windows = if pattern_len == 0 {
            text_len + 1
        } else if pattern_len > text_len {
            0
        } else {
            text_len - pattern_len + 1
        };
        let mut result = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        write_usize_array(&mut result, matches.clone());
        *output = TyrusRabinKarpInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(text_bytes).unwrap_or_default()),
            pattern: crate::strings::alloc_string(
                std::str::from_utf8(pattern_bytes).unwrap_or_default(),
            ),
            matches: result,
            comparison_count: windows.saturating_mul(pattern_len),
            shift_count: windows.saturating_sub(1),
            window_count: windows,
            hash_collision_count: 0,
            text_verification_count: matches.len(),
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boyer_moore(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Some(text), Some(pattern)) = (bytes(text, text_len), bytes(pattern, pattern_len))
        else {
            return;
        };
        write_usize_array(output, boyer_moore(text, pattern));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_boyer_moore_info(
    text: *const c_char,
    text_len: usize,
    pattern: *const c_char,
    pattern_len: usize,
    output: *mut TyrusBoyerMooreInfo,
) {
    unsafe {
        let (Some(text_bytes), Some(pattern_bytes), Some(output)) = (
            bytes(text, text_len),
            bytes(pattern, pattern_len),
            output.as_mut(),
        ) else {
            return;
        };
        let (matches, comparisons, table, shifts) = boyer_moore_info(text_bytes, pattern_bytes);
        let mut matches_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut table_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut shifts_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        write_usize_array(&mut matches_out, matches);
        write_pair_array(&mut table_out, table);
        write_usize_array(&mut shifts_out, shifts.clone());
        *output = TyrusBoyerMooreInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(text_bytes).unwrap_or_default()),
            pattern: crate::strings::alloc_string(
                std::str::from_utf8(pattern_bytes).unwrap_or_default(),
            ),
            matches: matches_out,
            comparison_count: comparisons,
            shift_count: shifts.len(),
            bad_character_table: table_out,
            applied_shifts: shifts_out,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_aho_corasick(
    text: *const c_char,
    text_len: usize,
    patterns: *const TyrusArray,
    output: *mut TyrusArray,
) {
    unsafe {
        let (Some(text), Some(patterns)) = (bytes(text, text_len), patterns.as_ref()) else {
            return;
        };
        if patterns.length > 0 && patterns.data.is_null() {
            return;
        }
        let pointers = if patterns.length == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(patterns.data.cast::<TyrusString>(), patterns.length)
        };
        let pattern_values = pointers
            .iter()
            .map(|pattern| {
                if pattern.is_null() {
                    Vec::new()
                } else {
                    pattern.as_str().as_bytes().to_vec()
                }
            })
            .collect::<Vec<_>>();
        write_pair_array(output, aho_corasick(text, &pattern_values));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_aho_corasick_info(
    text: *const c_char,
    text_len: usize,
    patterns: *const TyrusArray,
    output: *mut TyrusAhoCorasickInfo,
) {
    unsafe {
        let (Some(text_bytes), Some(patterns), Some(output)) =
            (bytes(text, text_len), patterns.as_ref(), output.as_mut())
        else {
            return;
        };
        if patterns.length > 0 && patterns.data.is_null() {
            return;
        }
        let pointers = if patterns.length == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(patterns.data.cast::<TyrusString>(), patterns.length)
        };
        let pattern_values = pointers
            .iter()
            .map(|pattern| {
                if pattern.is_null() {
                    Vec::new()
                } else {
                    pattern.as_str().as_bytes().to_vec()
                }
            })
            .collect::<Vec<_>>();
        let (matches, visited_states, transition_count, failure_follow_count) =
            aho_corasick_info(text_bytes, &pattern_values);
        let mut patterns_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut matches_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let mut states_out = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        clone_string_array(&mut patterns_out, pointers);
        write_pair_array(&mut matches_out, matches);
        write_usize_array(&mut states_out, visited_states);
        *output = TyrusAhoCorasickInfo {
            text: crate::strings::alloc_string(std::str::from_utf8(text_bytes).unwrap_or_default()),
            patterns: patterns_out,
            matches: matches_out,
            visited_states: states_out,
            transition_count,
            failure_follow_count,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_levenshtein(
    a: *const c_char,
    a_len: usize,
    b: *const c_char,
    b_len: usize,
    output: *mut usize,
) -> i32 {
    unsafe {
        let (Some(a), Some(b), Some(output)) = (
            string_parts(a, a_len),
            string_parts(b, b_len),
            output.as_mut(),
        ) else {
            return 0;
        };
        *output = levenshtein(a, b);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_algorithm_levenshtein_info(
    left: *const c_char,
    left_len: usize,
    right: *const c_char,
    right_len: usize,
    output: *mut TyrusEditDistanceInfo,
) {
    unsafe {
        let (Some(left_bytes), Some(right_bytes), Some(output)) = (
            bytes(left, left_len),
            bytes(right, right_len),
            output.as_mut(),
        ) else {
            return;
        };
        let (distance, evaluated_cell_count) = levenshtein_info(left_bytes, right_bytes);
        *output = TyrusEditDistanceInfo {
            left: crate::strings::alloc_string(std::str::from_utf8(left_bytes).unwrap_or_default()),
            right: crate::strings::alloc_string(
                std::str::from_utf8(right_bytes).unwrap_or_default(),
            ),
            distance,
            row_count: left_len + 1,
            column_count: right_len + 1,
            evaluated_cell_count,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_search_algorithms_cover_matches_and_empty_patterns() {
        let text = b"ababa";
        assert_eq!(kmp_search(text, b"aba"), vec![0, 2]);
        assert_eq!(z_algorithm(b"aabcaab"), vec![0, 1, 0, 0, 3, 1, 0]);
        assert_eq!(rabin_karp(text, b"ba"), vec![1, 3]);
        assert_eq!(boyer_moore(text, b"aba"), vec![0, 2]);
        assert_eq!(
            aho_corasick(text, &[b"aba".to_vec(), b"ba".to_vec()]),
            vec![(2, 0), (2, 1), (4, 0), (4, 1)]
        );
        assert_eq!(kmp_search(text, b""), vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn boyer_moore_info_records_the_bad_character_search() {
        let (matches, comparisons, table, shifts) = boyer_moore_info(b"ababa", b"aba");
        assert_eq!(matches, vec![0, 2]);
        assert_eq!(comparisons, 7);
        assert_eq!(table, vec![(b'a' as usize, 2), (b'b' as usize, 1)]);
        assert_eq!(shifts, vec![1, 1, 1]);
    }

    #[test]
    fn aho_corasick_info_records_states_transitions_and_failures() {
        let (matches, states, transitions, failures) =
            aho_corasick_info(b"ababa", &[b"aba".to_vec(), b"ba".to_vec()]);
        assert_eq!(matches, vec![(2, 0), (2, 1), (4, 0), (4, 1)]);
        assert_eq!(states, vec![1, 2, 3, 2, 3]);
        assert_eq!(transitions, 5);
        assert_eq!(failures, 2);
    }

    #[test]
    fn levenshtein_covers_empty_and_unicode_bytes() {
        assert_eq!(levenshtein(b"kitten", b"sitting"), 3);
        assert_eq!(levenshtein(b"", b"abc"), 3);
        assert_eq!(levenshtein("café".as_bytes(), "cafe".as_bytes()), 2);
    }

    #[test]
    fn levenshtein_info_reports_matrix_shape_without_retaining_it() {
        assert_eq!(levenshtein_info(b"kitten", b"sitting"), (3, 42));
    }
}
