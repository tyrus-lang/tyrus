//! Native `Set<T>` storage and C ABI operations, backed by a hash set with
//! insertion-order iteration (matching JavaScript `Set`).
//!
//! Each element is canonicalized to a byte vector before hashing: numeric and
//! other `Copy` values use their raw bytes (`kind` 0), while strings use their
//! UTF-8 content (`kind` 1) so equal strings compare equal regardless of pointer
//! identity.

use super::*;
use std::collections::HashSet;
use tyrus_string::TyrusString;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

pub struct TyrusSet {
    members: HashSet<Vec<u8>>,
    order: Vec<Vec<u8>>,
    elem_size: usize,
    kind: i32,
}

impl TyrusSet {
    unsafe fn key(&self, value: *const u8) -> Option<Vec<u8>> {
        unsafe {
            if value.is_null() {
                return None;
            }
            if self.kind == 1 {
                let stored = &*(value.cast::<TyrusString>());
                text(stored).ok().map(String::into_bytes)
            } else {
                Some(std::slice::from_raw_parts(value, self.elem_size).to_vec())
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_set_new(elem_size: usize, kind: i32) -> *mut TyrusSet {
    Box::into_raw(Box::new(TyrusSet {
        members: HashSet::new(),
        order: Vec::new(),
        elem_size,
        kind,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_set_add(set: *mut TyrusSet, value: *const u8) {
    unsafe {
        let Some(set) = set.as_mut() else {
            return;
        };
        let Some(key) = set.key(value) else {
            return;
        };
        if set.members.insert(key.clone()) {
            set.order.push(key);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_set_has(set: *const TyrusSet, value: *const u8) -> bool {
    unsafe {
        let Some(set) = set.as_ref() else {
            return false;
        };
        match set.key(value) {
            Some(key) => set.members.contains(&key),
            None => false,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_set_delete(set: *mut TyrusSet, value: *const u8) -> bool {
    unsafe {
        let Some(set) = set.as_mut() else {
            return false;
        };
        let Some(key) = set.key(value) else {
            return false;
        };
        if set.members.remove(&key) {
            set.order.retain(|entry| entry != &key);
            true
        } else {
            false
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_set_size(set: *const TyrusSet) -> usize {
    unsafe { set.as_ref().map_or(0, |set| set.members.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_set_values(set: *const TyrusSet, output: *mut TyrusArray) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let empty = TyrusArray {
            data: ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        let Some(set) = set.as_ref() else {
            *output = empty;
            return;
        };
        let length = set.order.len();
        let element_size = if set.kind == 1 {
            std::mem::size_of::<TyrusString>()
        } else {
            set.elem_size
        };
        let data = if length == 0 {
            ptr::null_mut()
        } else {
            malloc(length * element_size).cast::<u8>()
        };
        if length > 0 && data.is_null() {
            *output = empty;
            return;
        }
        for (index, entry) in set.order.iter().enumerate() {
            let slot = data.add(index * element_size);
            if set.kind == 1 {
                let text = String::from_utf8_lossy(entry);
                std::ptr::write(
                    slot.cast::<TyrusString>(),
                    crate::strings::alloc_string(&text),
                );
            } else {
                std::ptr::copy_nonoverlapping(entry.as_ptr(), slot, set.elem_size);
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
pub unsafe extern "C" fn tyrus_set_free(set: *mut TyrusSet) {
    unsafe {
        if !set.is_null() {
            drop(Box::from_raw(set));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn add_i32(set: *mut TyrusSet, value: i32) {
        unsafe {
            tyrus_set_add(set, (&value as *const i32).cast());
        }
    }

    unsafe fn has_i32(set: *const TyrusSet, value: i32) -> bool {
        unsafe { tyrus_set_has(set, (&value as *const i32).cast()) }
    }

    #[test]
    fn enforces_uniqueness_and_membership() {
        unsafe {
            let set = tyrus_set_new(std::mem::size_of::<i32>(), 0);
            add_i32(set, 1);
            add_i32(set, 1);
            add_i32(set, 2);
            assert_eq!(tyrus_set_size(set), 2);
            assert!(has_i32(set, 1));
            assert!(!has_i32(set, 9));
            let value = 1;
            assert!(tyrus_set_delete(set, (&value as *const i32).cast()));
            assert!(!has_i32(set, 1));
            assert_eq!(tyrus_set_size(set), 1);
            tyrus_set_free(set);
        }
    }

    #[test]
    fn hashes_strings_by_content() {
        unsafe {
            let set = tyrus_set_new(std::mem::size_of::<TyrusString>(), 1);
            let a = TyrusString::literal("alpha");
            let a_again = TyrusString::literal("alpha");
            let b = TyrusString::literal("beta");
            let a_ptr = a.borrow();
            let a2_ptr = a_again.borrow();
            let b_ptr = b.borrow();
            tyrus_set_add(set, (&a_ptr as *const TyrusString).cast());
            tyrus_set_add(set, (&a2_ptr as *const TyrusString).cast());
            tyrus_set_add(set, (&b_ptr as *const TyrusString).cast());
            assert_eq!(tyrus_set_size(set), 2);

            let mut array = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            tyrus_set_values(set, &mut array);
            assert_eq!(array.length, 2);
            let slots = array.data.cast::<TyrusString>();
            assert_eq!(text(&*slots).unwrap(), "alpha");
            assert_eq!(text(&*slots.add(1)).unwrap(), "beta");
            for index in 0..array.length {
                std::mem::replace(&mut *slots.add(index), TyrusString::null()).release();
            }
            libc::free(array.data.cast());
            tyrus_set_free(set);
        }
    }
}
