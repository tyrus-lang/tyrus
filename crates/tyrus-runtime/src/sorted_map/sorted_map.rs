use super::sorted_avl::{AvlNode, delete, find, insert, max_key, min_key};
use super::*;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub struct TyrusSortedMap {
    root: Option<Box<AvlNode>>,
    len: usize,
    key_size: usize,
    value_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

fn with_compare(map: &TyrusSortedMap) -> impl Fn(&[u8], &[u8]) -> i32 + use<> {
    let compare_fn = map.compare;
    move |a: &[u8], b: &[u8]| match compare_fn {
        Some(f) => unsafe { f(a.as_ptr(), b.as_ptr()) },
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_new(
    key_size: usize,
    value_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusSortedMap {
    let map = Box::new(TyrusSortedMap {
        root: None,
        len: 0,
        key_size,
        value_size,
        compare,
    });
    Box::into_raw(map)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_set(
    map: *mut TyrusSortedMap,
    key: *const u8,
    value: *const u8,
) {
    unsafe {
        let map = &mut *map;
        let key_bytes = std::slice::from_raw_parts(key, map.key_size).to_vec();
        let value_bytes = std::slice::from_raw_parts(value, map.value_size).to_vec();
        let (root, inserted) = insert(
            map.root.take(),
            &key_bytes,
            Some(value_bytes.into_boxed_slice()),
            true,
            &with_compare(map),
        );
        map.root = root;
        if inserted {
            map.len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_get(
    map: *const TyrusSortedMap,
    key: *const u8,
) -> *mut u8 {
    unsafe {
        let map = &*map;
        let key_bytes = std::slice::from_raw_parts(key, map.key_size);
        let Some(value) = find(&map.root, key_bytes, &with_compare(map)) else {
            return ptr::null_mut();
        };
        let storage = malloc(map.value_size.max(1)).cast::<u8>();
        if storage.is_null() {
            return ptr::null_mut();
        }
        std::ptr::copy_nonoverlapping(value.as_ptr(), storage, map.value_size);
        storage
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_has(map: *const TyrusSortedMap, key: *const u8) -> bool {
    unsafe {
        let map = &*map;
        let key_bytes = std::slice::from_raw_parts(key, map.key_size);
        find(&map.root, key_bytes, &with_compare(map)).is_some()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_delete(map: *mut TyrusSortedMap, key: *const u8) -> bool {
    unsafe {
        let map = &mut *map;
        let key_bytes = std::slice::from_raw_parts(key, map.key_size);
        let result = delete(map.root.take(), key_bytes, &with_compare(map));
        map.root = result.root;
        if result.deleted {
            map.len -= 1;
        }
        result.deleted
    }
}

fn extreme_key(map: &TyrusSortedMap, key: Option<&[u8]>) -> *mut u8 {
    let Some(key) = key else {
        return ptr::null_mut();
    };
    let storage = unsafe { malloc(map.key_size.max(1)).cast::<u8>() };
    if storage.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(key.as_ptr(), storage, map.key_size);
    }
    storage
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_min(map: *const TyrusSortedMap) -> *mut u8 {
    unsafe {
        let map = &*map;
        extreme_key(map, min_key(&map.root))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_max(map: *const TyrusSortedMap) -> *mut u8 {
    unsafe {
        let map = &*map;
        extreme_key(map, max_key(&map.root))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_len(map: *const TyrusSortedMap) -> usize {
    unsafe { (*map).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_is_empty(map: *const TyrusSortedMap) -> bool {
    unsafe { (*map).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_clear(map: *mut TyrusSortedMap) {
    unsafe {
        let map = &mut *map;
        map.root = None;
        map.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_map_free(map: *mut TyrusSortedMap) {
    unsafe {
        if !map.is_null() {
            drop(Box::from_raw(map));
        }
    }
}
