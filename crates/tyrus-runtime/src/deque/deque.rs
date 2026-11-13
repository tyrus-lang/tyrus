use crate::tyrus_panic;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: Deque allocation failed".as_ptr()) }
}

/// Opaque deque backed by a circular byte buffer. Elements are fixed-size values
/// whose representation is copied by the compiler at the ABI boundary.
pub struct TyrusDeque {
    data: *mut u8,
    len: usize,
    capacity: usize,
    head: usize,
    element_size: usize,
}

impl TyrusDeque {
    unsafe fn slot(&self, logical_index: usize) -> *mut u8 {
        unsafe {
            debug_assert!(logical_index < self.len);
            self.data
                .add(((self.head + logical_index) % self.capacity) * self.element_size)
        }
    }

    unsafe fn grow(&mut self) {
        unsafe {
            let new_capacity = if self.capacity == 0 {
                4
            } else {
                self.capacity
                    .checked_mul(2)
                    .unwrap_or_else(|| allocation_failed())
            };
            let bytes = new_capacity
                .checked_mul(self.element_size)
                .unwrap_or_else(|| allocation_failed());
            let new_data = malloc(bytes).cast::<u8>();
            if new_data.is_null() {
                allocation_failed();
            }
            for index in 0..self.len {
                ptr::copy_nonoverlapping(
                    self.slot(index),
                    new_data.add(index * self.element_size),
                    self.element_size,
                );
            }
            if !self.data.is_null() {
                free(self.data.cast());
            }
            self.data = new_data;
            self.capacity = new_capacity;
            self.head = 0;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_new(element_size: usize) -> *mut TyrusDeque {
    if element_size == 0 {
        allocation_failed();
    }
    Box::into_raw(Box::new(TyrusDeque {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
        head: 0,
        element_size,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_len(deque: *const TyrusDeque) -> usize {
    unsafe { (*deque).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_is_empty(deque: *const TyrusDeque) -> bool {
    unsafe { (*deque).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_push_front(deque: *mut TyrusDeque, element: *const u8) {
    unsafe {
        let deque = &mut *deque;
        if deque.len == deque.capacity {
            deque.grow();
        }
        deque.head = (deque.head + deque.capacity - 1) % deque.capacity;
        ptr::copy_nonoverlapping(
            element,
            deque.data.add(deque.head * deque.element_size),
            deque.element_size,
        );
        deque.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_push_back(deque: *mut TyrusDeque, element: *const u8) {
    unsafe {
        let deque = &mut *deque;
        if deque.len == deque.capacity {
            deque.grow();
        }
        let tail = (deque.head + deque.len) % deque.capacity;
        ptr::copy_nonoverlapping(
            element,
            deque.data.add(tail * deque.element_size),
            deque.element_size,
        );
        deque.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_pop_front(deque: *mut TyrusDeque, output: *mut u8) -> bool {
    unsafe {
        let deque = &mut *deque;
        if deque.len == 0 {
            return false;
        }
        ptr::copy_nonoverlapping(
            deque.data.add(deque.head * deque.element_size),
            output,
            deque.element_size,
        );
        deque.head = (deque.head + 1) % deque.capacity;
        deque.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_pop_back(deque: *mut TyrusDeque, output: *mut u8) -> bool {
    unsafe {
        let deque = &mut *deque;
        if deque.len == 0 {
            return false;
        }
        let tail = (deque.head + deque.len - 1) % deque.capacity;
        ptr::copy_nonoverlapping(
            deque.data.add(tail * deque.element_size),
            output,
            deque.element_size,
        );
        deque.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_peek_front(deque: *const TyrusDeque, output: *mut u8) -> bool {
    unsafe {
        let deque = &*deque;
        if deque.len == 0 {
            return false;
        }
        ptr::copy_nonoverlapping(
            deque.data.add(deque.head * deque.element_size),
            output,
            deque.element_size,
        );
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_peek_back(deque: *const TyrusDeque, output: *mut u8) -> bool {
    unsafe {
        let deque = &*deque;
        if deque.len == 0 {
            return false;
        }
        let tail = (deque.head + deque.len - 1) % deque.capacity;
        ptr::copy_nonoverlapping(
            deque.data.add(tail * deque.element_size),
            output,
            deque.element_size,
        );
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_clear(deque: *mut TyrusDeque) {
    unsafe {
        let deque = &mut *deque;
        deque.len = 0;
        deque.head = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_keys(
    deque: *const TyrusDeque,
    output: *mut crate::TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(deque) = deque.as_ref() else {
            *output = crate::TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };
        let bytes = deque
            .len
            .checked_mul(deque.element_size)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }
        for index in 0..deque.len {
            ptr::copy_nonoverlapping(
                deque.slot(index),
                data.add(index * deque.element_size),
                deque.element_size,
            );
        }
        *output = crate::TyrusArray {
            data,
            length: deque.len,
            capacity: deque.len,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_deque_free(deque: *mut TyrusDeque) {
    unsafe {
        if !deque.is_null() {
            let deque = Box::from_raw(deque);
            if !deque.data.is_null() {
                free(deque.data.cast());
            }
        }
    }
}
