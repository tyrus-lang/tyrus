//! Native singly linked list runtime for `tyrus:algorithm`.
//!
//! # FFI safety contract
//!
//! All pointers must be non-null and point to valid `TyrusLinkedList`
//! allocations. Elements are fixed-size scalar values whose representation is
//! copied at the ABI boundary. `popFront`, `popBack`, `front` and `back` return
//! whether a value was produced and copy it into `output`.

use crate::tyrus_panic;

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: LinkedList allocation failed".as_ptr()) }
}

struct TyrusLinkedListNode {
    value: Vec<u8>,
    next: *mut TyrusLinkedListNode,
}

pub struct TyrusLinkedList {
    head: *mut TyrusLinkedListNode,
    tail: *mut TyrusLinkedListNode,
    len: usize,
    element_size: usize,
}

unsafe fn new_node(element_size: usize, value: *const u8) -> *mut TyrusLinkedListNode {
    unsafe {
        if element_size == 0 {
            allocation_failed();
        }
        let node = Box::new(TyrusLinkedListNode {
            value: vec![0u8; element_size],
            next: std::ptr::null_mut(),
        });
        let raw = Box::into_raw(node);
        std::ptr::copy_nonoverlapping(value, (*raw).value.as_mut_ptr(), element_size);
        raw
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_new(element_size: usize) -> *mut TyrusLinkedList {
    if element_size == 0 {
        allocation_failed();
    }
    Box::into_raw(Box::new(TyrusLinkedList {
        head: std::ptr::null_mut(),
        tail: std::ptr::null_mut(),
        len: 0,
        element_size,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_from_array(
    element_size: usize,
    data: *const u8,
    length: usize,
) -> *mut TyrusLinkedList {
    unsafe {
        let list = Box::new(TyrusLinkedList {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            len: 0,
            element_size,
        });
        let raw = Box::into_raw(list);
        for index in 0..length {
            let element = data.add(index * element_size);
            tyrus_linked_list_push_back(raw, element);
        }
        raw
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_push_front(list: *mut TyrusLinkedList, item: *const u8) {
    unsafe {
        let list = &mut *list;
        let node = new_node(list.element_size, item);
        (*node).next = list.head;
        list.head = node;
        if list.tail.is_null() {
            list.tail = node;
        }
        list.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_push_back(list: *mut TyrusLinkedList, item: *const u8) {
    unsafe {
        let list = &mut *list;
        let node = new_node(list.element_size, item);
        if list.tail.is_null() {
            list.head = node;
        } else {
            (*list.tail).next = node;
        }
        list.tail = node;
        list.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_pop_front(
    list: *mut TyrusLinkedList,
    output: *mut u8,
) -> bool {
    unsafe {
        let list = &mut *list;
        if list.head.is_null() {
            return false;
        }
        let node = Box::from_raw(list.head);
        std::ptr::copy_nonoverlapping(node.value.as_ptr(), output, list.element_size);
        list.head = node.next;
        if list.head.is_null() {
            list.tail = std::ptr::null_mut();
        }
        list.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_pop_back(
    list: *mut TyrusLinkedList,
    output: *mut u8,
) -> bool {
    unsafe {
        let list = &mut *list;
        if list.head.is_null() {
            return false;
        }
        if list.head == list.tail {
            return tyrus_linked_list_pop_front(list, output);
        }
        let mut cursor = list.head;
        while (*cursor).next != list.tail {
            cursor = (*cursor).next;
        }
        let tail = Box::from_raw(list.tail);
        std::ptr::copy_nonoverlapping(tail.value.as_ptr(), output, list.element_size);
        (*cursor).next = std::ptr::null_mut();
        list.tail = cursor;
        list.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_front(
    list: *const TyrusLinkedList,
    output: *mut u8,
) -> bool {
    unsafe {
        let list = &*list;
        if list.head.is_null() {
            return false;
        }
        std::ptr::copy_nonoverlapping((*list.head).value.as_ptr(), output, list.element_size);
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_back(
    list: *const TyrusLinkedList,
    output: *mut u8,
) -> bool {
    unsafe {
        let list = &*list;
        if list.tail.is_null() {
            return false;
        }
        std::ptr::copy_nonoverlapping((*list.tail).value.as_ptr(), output, list.element_size);
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_len(list: *const TyrusLinkedList) -> usize {
    unsafe { (*list).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_is_empty(list: *const TyrusLinkedList) -> bool {
    unsafe { (*list).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_clear(list: *mut TyrusLinkedList) {
    unsafe {
        let list = &mut *list;
        let mut cursor = list.head;
        while !cursor.is_null() {
            let node = Box::from_raw(cursor);
            cursor = node.next;
        }
        list.head = std::ptr::null_mut();
        list.tail = std::ptr::null_mut();
        list.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_linked_list_free(list: *mut TyrusLinkedList) {
    unsafe {
        if !list.is_null() {
            let list = Box::from_raw(list);
            let mut cursor = list.head;
            while !cursor.is_null() {
                let node = Box::from_raw(cursor);
                cursor = node.next;
            }
        }
    }
}
