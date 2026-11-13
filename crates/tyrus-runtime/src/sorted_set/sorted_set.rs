use super::sorted_avl::{AvlNode, contains, delete, insert, max_key, min_key};
use super::*;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub struct TyrusSortedSet {
    root: Option<Box<AvlNode>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

fn with_compare(tree: &TyrusSortedSet) -> impl Fn(&[u8], &[u8]) -> i32 + use<> {
    let compare_fn = tree.compare;
    move |a: &[u8], b: &[u8]| match compare_fn {
        Some(f) => unsafe { f(a.as_ptr(), b.as_ptr()) },
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusSortedSet {
    let set = Box::new(TyrusSortedSet {
        root: None,
        len: 0,
        element_size,
        compare,
    });
    Box::into_raw(set)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_insert(
    set: *mut TyrusSortedSet,
    element: *const u8,
) -> bool {
    unsafe {
        let set = &mut *set;
        let key = std::slice::from_raw_parts(element, set.element_size);
        let (root, inserted) = insert(set.root.take(), key, None, false, &with_compare(set));
        set.root = root;
        if inserted {
            set.len += 1;
        }
        inserted
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_remove(
    set: *mut TyrusSortedSet,
    element: *const u8,
) -> bool {
    unsafe {
        let set = &mut *set;
        let key = std::slice::from_raw_parts(element, set.element_size);
        let result = delete(set.root.take(), key, &with_compare(set));
        set.root = result.root;
        if result.deleted {
            set.len -= 1;
        }
        result.deleted
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_contains(
    set: *const TyrusSortedSet,
    element: *const u8,
) -> bool {
    unsafe {
        let set = &*set;
        let key = std::slice::from_raw_parts(element, set.element_size);
        contains(&set.root, key, &with_compare(set))
    }
}

fn extreme(set: &TyrusSortedSet, key: Option<&[u8]>) -> *mut u8 {
    let Some(key) = key else {
        return ptr::null_mut();
    };
    let storage = unsafe { malloc(set.element_size.max(1)).cast::<u8>() };
    if storage.is_null() {
        return ptr::null_mut();
    }
    unsafe {
        std::ptr::copy_nonoverlapping(key.as_ptr(), storage, set.element_size);
    }
    storage
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_min(set: *const TyrusSortedSet) -> *mut u8 {
    unsafe {
        let set = &*set;
        extreme(set, min_key(&set.root))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_max(set: *const TyrusSortedSet) -> *mut u8 {
    unsafe {
        let set = &*set;
        extreme(set, max_key(&set.root))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_len(set: *const TyrusSortedSet) -> usize {
    unsafe { (*set).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_is_empty(set: *const TyrusSortedSet) -> bool {
    unsafe { (*set).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_clear(set: *mut TyrusSortedSet) {
    unsafe {
        let set = &mut *set;
        set.root = None;
        set.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_sorted_set_free(set: *mut TyrusSortedSet) {
    unsafe {
        if !set.is_null() {
            drop(Box::from_raw(set));
        }
    }
}
