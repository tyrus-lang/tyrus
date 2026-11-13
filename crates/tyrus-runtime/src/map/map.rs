//! Native dynamic map (`Record<string, T>` and `Map<K, V>`) storage and C ABI
//! operations, backed by a hash map with insertion-order iteration.
//!
//! Keys are canonicalized to a byte vector before hashing: string keys use their
//! UTF-8 content (`key_kind` 1) so equal strings compare equal regardless of
//! pointer identity, while numeric and boolean keys use their raw bytes
//! (`key_kind` 0). Values are stored as opaque byte blobs; a read returns a
//! freshly allocated copy, matching the compiler's nullable-pointer `T?`.

use super::*;
use std::collections::HashMap;
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub struct TyrusMap {
    values: HashMap<Vec<u8>, Vec<u8>>,
    order: Vec<Vec<u8>>,
    key_size: usize,
    key_kind: i32,
    value_size: usize,
    grouped_element_size: Option<usize>,
}

impl TyrusMap {
    unsafe fn canonical_key(&self, key: *const u8) -> Option<Vec<u8>> {
        unsafe {
            if key.is_null() {
                return None;
            }
            if self.key_kind == 1 {
                let stored = &*(key.cast::<TyrusString>());
                text(stored).ok().map(String::into_bytes)
            } else if self.key_kind == 2 {
                let value = *(key.cast::<f32>());
                let bits = if value == 0.0 { 0 } else { value.to_bits() };
                Some({ bits }.to_ne_bytes().to_vec())
            } else if self.key_kind == 3 {
                let value = *(key.cast::<f64>());
                let bits = if value == 0.0 { 0 } else { value.to_bits() };
                Some(bits.to_ne_bytes().to_vec())
            } else {
                Some(std::slice::from_raw_parts(key, self.key_size).to_vec())
            }
        }
    }

    unsafe fn collect(
        &self,
        output: *mut TyrusArray,
        pick: impl Fn(&Vec<u8>) -> &Vec<u8>,
        element_size: usize,
        as_strings: bool,
    ) {
        unsafe {
            let Some(output) = output.as_mut() else {
                return;
            };
            let length = self.order.len();
            let stride = if as_strings {
                std::mem::size_of::<TyrusString>()
            } else {
                element_size
            };
            let data = if length == 0 {
                ptr::null_mut()
            } else {
                malloc(length * stride).cast::<u8>()
            };
            if length > 0 && data.is_null() {
                *output = TyrusArray {
                    data: ptr::null_mut(),
                    length: 0,
                    capacity: 0,
                };
                return;
            }
            for (index, key) in self.order.iter().enumerate() {
                let bytes = pick(key);
                let slot = data.add(index * stride);
                if as_strings {
                    let text = String::from_utf8_lossy(bytes);
                    std::ptr::write(
                        slot.cast::<TyrusString>(),
                        crate::strings::alloc_string(&text),
                    );
                } else {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), slot, element_size);
                }
            }
            *output = TyrusArray {
                data,
                length,
                capacity: length,
            };
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_map_new(
    key_size: usize,
    key_kind: i32,
    value_size: usize,
) -> *mut TyrusMap {
    Box::into_raw(Box::new(TyrusMap {
        values: HashMap::new(),
        order: Vec::new(),
        key_size,
        key_kind,
        value_size,
        grouped_element_size: None,
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_map_new_grouped(
    key_size: usize,
    key_kind: i32,
    element_size: usize,
) -> *mut TyrusMap {
    Box::into_raw(Box::new(TyrusMap {
        values: HashMap::new(),
        order: Vec::new(),
        key_size,
        key_kind,
        value_size: std::mem::size_of::<TyrusArray>(),
        grouped_element_size: Some(element_size),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_group_push(
    map: *mut TyrusMap,
    key: *const u8,
    value: *const u8,
) -> i32 {
    unsafe {
        let Some(map) = map.as_mut() else {
            return -1;
        };
        let Some(element_size) = map.grouped_element_size else {
            return -1;
        };
        let Some(key) = map.canonical_key(key) else {
            return -1;
        };
        if value.is_null() {
            return -1;
        }
        let entry = map.values.entry(key.clone()).or_insert_with(|| {
            map.order.push(key);
            Vec::new()
        });
        entry.extend_from_slice(std::slice::from_raw_parts(value, element_size));
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_insert(map: *mut TyrusMap, key: *const u8, value: *const u8) {
    unsafe {
        let Some(map) = map.as_mut() else {
            return;
        };
        let (Some(key), false) = (map.canonical_key(key), value.is_null()) else {
            return;
        };
        let bytes = std::slice::from_raw_parts(value, map.value_size).to_vec();
        if map.values.insert(key.clone(), bytes).is_none() {
            map.order.push(key);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_get(map: *const TyrusMap, key: *const u8) -> *mut u8 {
    unsafe {
        let Some(map) = map.as_ref() else {
            return ptr::null_mut();
        };
        let Some(key) = map.canonical_key(key) else {
            return ptr::null_mut();
        };
        let Some(bytes) = map.values.get(&key) else {
            return ptr::null_mut();
        };
        if let Some(element_size) = map.grouped_element_size {
            if element_size == 0 || bytes.len() % element_size != 0 {
                return ptr::null_mut();
            }
            let data_bytes = bytes.len();
            let data = if data_bytes == 0 {
                ptr::null_mut()
            } else {
                malloc(data_bytes).cast::<u8>()
            };
            if data_bytes != 0 && data.is_null() {
                return ptr::null_mut();
            }
            if data_bytes != 0 {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, data_bytes);
            }
            let output = Box::new(TyrusArray {
                data,
                length: bytes.len() / element_size,
                capacity: bytes.len() / element_size,
            });
            return Box::into_raw(output).cast::<u8>();
        }
        let storage = malloc(map.value_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), storage, map.value_size);
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_has(map: *const TyrusMap, key: *const u8) -> bool {
    unsafe {
        let Some(map) = map.as_ref() else {
            return false;
        };
        match map.canonical_key(key) {
            Some(key) => map.values.contains_key(&key),
            None => false,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_delete(map: *mut TyrusMap, key: *const u8) -> bool {
    unsafe {
        let Some(map) = map.as_mut() else {
            return false;
        };
        let Some(key) = map.canonical_key(key) else {
            return false;
        };
        if map.values.remove(&key).is_some() {
            map.order.retain(|entry| entry != &key);
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_size(map: *const TyrusMap) -> usize {
    unsafe { map.as_ref().map_or(0, |map| map.values.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_keys(map: *const TyrusMap, output: *mut TyrusArray) {
    unsafe {
        let Some(map) = map.as_ref() else {
            return;
        };
        map.collect(output, |key| key, map.key_size, map.key_kind == 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_values(map: *const TyrusMap, output: *mut TyrusArray) {
    unsafe {
        let Some(map) = map.as_ref() else {
            return;
        };
        let values = &map.values;
        map.collect(
            output,
            |key| values.get(key).expect("ordered key must have a value"),
            map.value_size,
            false,
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_map_free(map: *mut TyrusMap) {
    unsafe {
        if !map.is_null() {
            drop(Box::from_raw(map));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn free_value(ptr: *mut u8) {
        unsafe {
            if !ptr.is_null() {
                libc::free(ptr.cast());
            }
        }
    }

    unsafe fn free_collected(array: &TyrusArray, owns_strings: bool) {
        unsafe {
            if owns_strings {
                let slots = array.data as *mut TyrusString;
                for index in 0..array.length {
                    std::mem::replace(&mut *slots.add(index), TyrusString::null()).release();
                }
            }
            if !array.data.is_null() {
                libc::free(array.data.cast());
            }
        }
    }

    unsafe fn insert_str_i32(map: *mut TyrusMap, key: &str, value: i32) {
        unsafe {
            let key = crate::strings::alloc_string(key);
            let key_ptr = key.borrow();
            tyrus_map_insert(
                map,
                (&key_ptr as *const TyrusString).cast(),
                (&value as *const i32).cast(),
            );
            key.release();
        }
    }

    fn string_map() -> *mut TyrusMap {
        tyrus_map_new(
            std::mem::size_of::<TyrusString>(),
            1,
            std::mem::size_of::<i32>(),
        )
    }

    #[test]
    fn inserts_reads_and_reports_presence() {
        unsafe {
            let map = string_map();
            insert_str_i32(map, "a", 1);
            insert_str_i32(map, "b", 2);
            assert_eq!(tyrus_map_size(map), 2);

            let present = TyrusString::literal("a");
            let present_ptr = present.borrow();
            let value = tyrus_map_get(map, (&present_ptr as *const TyrusString).cast());
            assert!(!value.is_null());
            assert_eq!(*value.cast::<i32>(), 1);
            free_value(value);

            let absent = TyrusString::literal("z");
            let absent_ptr = absent.borrow();
            assert!(tyrus_map_get(map, (&absent_ptr as *const TyrusString).cast()).is_null());
            assert!(tyrus_map_has(
                map,
                (&present_ptr as *const TyrusString).cast()
            ));
            tyrus_map_free(map);
        }
    }

    #[test]
    fn overwrites_and_deletes_preserving_order() {
        unsafe {
            let map = string_map();
            insert_str_i32(map, "a", 1);
            insert_str_i32(map, "b", 2);
            insert_str_i32(map, "a", 9);
            assert_eq!(tyrus_map_size(map), 2);

            let key = TyrusString::literal("a");
            let key_ptr = key.borrow();
            let value = tyrus_map_get(map, (&key_ptr as *const TyrusString).cast());
            assert_eq!(*value.cast::<i32>(), 9);
            free_value(value);
            assert!(tyrus_map_delete(
                map,
                (&key_ptr as *const TyrusString).cast()
            ));
            assert!(!tyrus_map_delete(
                map,
                (&key_ptr as *const TyrusString).cast()
            ));
            assert_eq!(tyrus_map_size(map), 1);
            tyrus_map_free(map);
        }
    }

    #[test]
    fn collects_keys_and_values_in_order() {
        unsafe {
            let map = string_map();
            insert_str_i32(map, "a", 1);
            insert_str_i32(map, "b", 2);
            let mut keys = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_map_keys(map, &mut keys);
            assert_eq!(keys.length, 2);
            let slots = keys.data.cast::<TyrusString>();
            assert_eq!(text(&*slots).unwrap(), "a");
            assert_eq!(text(&*slots.add(1)).unwrap(), "b");

            let mut values = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_map_values(map, &mut values);
            let value_slots = values.data.cast::<i32>();
            assert_eq!(*value_slots, 1);
            assert_eq!(*value_slots.add(1), 2);
            free_collected(&keys, true);
            free_collected(&values, false);
            tyrus_map_free(map);
        }
    }

    #[test]
    fn supports_numeric_keys() {
        unsafe {
            let map = tyrus_map_new(std::mem::size_of::<i32>(), 0, std::mem::size_of::<i32>());
            let key = 7_i32;
            let value = 70_i32;
            tyrus_map_insert(
                map,
                (&key as *const i32).cast(),
                (&value as *const i32).cast(),
            );
            assert!(tyrus_map_has(map, (&key as *const i32).cast()));
            let got = tyrus_map_get(map, (&key as *const i32).cast());
            assert_eq!(*got.cast::<i32>(), 70);
            free_value(got);
            tyrus_map_free(map);
        }
    }

    #[test]
    fn grouped_maps_preserve_order_within_each_key() {
        unsafe {
            let map =
                tyrus_map_new_grouped(std::mem::size_of::<i32>(), 0, std::mem::size_of::<i32>());
            let key = 1_i32;
            let first = 3_i32;
            let second = 5_i32;
            assert_eq!(
                tyrus_map_group_push(
                    map,
                    (&key as *const i32).cast(),
                    (&first as *const i32).cast()
                ),
                0
            );
            assert_eq!(
                tyrus_map_group_push(
                    map,
                    (&key as *const i32).cast(),
                    (&second as *const i32).cast()
                ),
                0
            );
            let value = tyrus_map_get(map, (&key as *const i32).cast()).cast::<TyrusArray>();
            assert!(!value.is_null());
            let array = &*value;
            assert_eq!(array.length, 2);
            assert_eq!(*array.data.cast::<i32>(), 3);
            assert_eq!(*array.data.add(std::mem::size_of::<i32>()).cast::<i32>(), 5);
            libc::free(array.data.cast());
            libc::free(value.cast());
            tyrus_map_free(map);
        }
    }
}
