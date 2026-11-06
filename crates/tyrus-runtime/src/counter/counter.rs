use crate::TyrusArray;
use crate::tyrus_panic;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: Counter allocation failed".as_ptr()) }
}

fn invalid_count() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: Counter count must be greater than zero".as_ptr()) }
}

fn count_overflow() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: Counter count overflow".as_ptr()) }
}

pub struct TyrusCounter {
    entries: HashMap<Vec<u8>, i32>,
    order: Vec<Vec<u8>>,
    element_size: usize,
}

impl TyrusCounter {
    unsafe fn canonical_key(&self, key: *const u8) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(key, self.element_size).to_vec() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_new(element_size: usize) -> *mut TyrusCounter {
    let counter = Box::new(TyrusCounter {
        entries: HashMap::new(),
        order: Vec::new(),
        element_size,
    });
    Box::into_raw(counter)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_from_array(
    element_size: usize,
    data: *const u8,
    length: usize,
) -> *mut TyrusCounter {
    unsafe {
        let mut counter = Box::new(TyrusCounter {
            entries: HashMap::new(),
            order: Vec::new(),
            element_size,
        });
        for i in 0..length {
            let key = std::slice::from_raw_parts(data.add(i * element_size), element_size).to_vec();
            let entry = counter.entries.entry(key.clone()).or_insert(0);
            *entry += 1;
            if *entry == 1 {
                counter.order.push(key);
            }
        }
        Box::into_raw(counter)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_add(
    counter: *mut TyrusCounter,
    item: *const u8,
    count: i32,
) {
    unsafe {
        if count <= 0 {
            invalid_count();
        }
        let counter = &mut *counter;
        let key = counter.canonical_key(item);
        let entry = counter.entries.entry(key.clone()).or_insert(0);
        let was_zero = *entry == 0;
        *entry = entry.checked_add(count).unwrap_or_else(|| count_overflow());
        if was_zero {
            counter.order.push(key);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_remove(
    counter: *mut TyrusCounter,
    item: *const u8,
    count: i32,
) {
    unsafe {
        if count <= 0 {
            invalid_count();
        }
        let counter = &mut *counter;
        let key = counter.canonical_key(item);
        let next = counter
            .entries
            .get(&key)
            .copied()
            .unwrap_or(0)
            .checked_sub(count)
            .unwrap_or(i32::MIN);
        if next <= 0 {
            counter.entries.remove(&key);
            counter.order.retain(|k| k != &key);
        } else {
            *counter
                .entries
                .get_mut(&key)
                .expect("counter entry must exist") = next;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_count(counter: *const TyrusCounter, item: *const u8) -> i32 {
    unsafe {
        let counter = &*counter;
        let key = counter.canonical_key(item);
        *counter.entries.get(&key).unwrap_or(&0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_total(counter: *const TyrusCounter) -> i32 {
    unsafe {
        let counter = &*counter;
        counter
            .entries
            .values()
            .try_fold(0_i32, |total, count| total.checked_add(*count))
            .unwrap_or_else(|| count_overflow())
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_unique(counter: *const TyrusCounter) -> usize {
    unsafe {
        let counter = &*counter;
        counter.entries.len()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_is_empty(counter: *const TyrusCounter) -> bool {
    unsafe {
        let counter = &*counter;
        counter.entries.is_empty()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_clear(counter: *mut TyrusCounter) {
    unsafe {
        let counter = &mut *counter;
        counter.entries.clear();
        counter.order.clear();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_most_common(
    counter: *const TyrusCounter,
    k: usize,
    pair_stride: usize,
    count_offset: usize,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(counter) = counter.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };

        let mut pairs: Vec<(&Vec<u8>, &i32)> = counter
            .order
            .iter()
            .filter_map(|key| counter.entries.get(key).map(|count| (key, count)))
            .collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));

        let take = k.min(pairs.len());
        let bytes = take
            .checked_mul(pair_stride)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }

        for (i, (key, count)) in pairs.iter().take(take).enumerate() {
            let slot = data.add(i * pair_stride);
            ptr::copy_nonoverlapping(key.as_ptr(), slot, counter.element_size);
            ptr::copy_nonoverlapping(
                (*count).to_le_bytes().as_ptr(),
                slot.add(count_offset),
                std::mem::size_of::<i32>(),
            );
        }

        *output = TyrusArray {
            data,
            length: take,
            capacity: take,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_items(
    counter: *const TyrusCounter,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(counter) = counter.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };

        let len = counter.order.len();
        let pair_size = counter.element_size + std::mem::size_of::<i32>();
        let bytes = len
            .checked_mul(pair_size)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }

        for (i, key) in counter.order.iter().enumerate() {
            let slot = data.add(i * pair_size);
            ptr::copy_nonoverlapping(key.as_ptr(), slot, counter.element_size);
            let count = *counter.entries.get(key).unwrap_or(&0);
            ptr::copy_nonoverlapping(
                count.to_le_bytes().as_ptr(),
                slot.add(counter.element_size),
                std::mem::size_of::<i32>(),
            );
        }

        *output = TyrusArray {
            data,
            length: len,
            capacity: len,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_keys(counter: *const TyrusCounter, output: *mut TyrusArray) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(counter) = counter.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };

        let len = counter.order.len();
        let bytes = len
            .checked_mul(counter.element_size)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }

        for (i, key) in counter.order.iter().enumerate() {
            let slot = data.add(i * counter.element_size);
            ptr::copy_nonoverlapping(key.as_ptr(), slot, counter.element_size);
        }

        *output = TyrusArray {
            data,
            length: len,
            capacity: len,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_counter_free(counter: *mut TyrusCounter) {
    unsafe {
        if !counter.is_null() {
            drop(Box::from_raw(counter));
        }
    }
}
