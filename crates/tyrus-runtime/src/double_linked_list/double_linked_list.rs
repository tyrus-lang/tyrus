//! Native doubly linked list runtime for `tyrus:algorithm`.
//!
//! # FFI safety contract
//!
//! All pointers must be non-null and point to valid `TyrusDoubleLinkedList`
//! allocations. Elements are fixed-size scalar values whose representation is
//! copied at the ABI boundary. `popFront`, `popBack`, `front` and `back` return
//! whether a value was produced and copy it into `output`.

use crate::tyrus_panic;

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: DoubleLinkedList allocation failed".as_ptr()) }
}

struct TyrusDoubleLinkedListNode {
    value: Vec<u8>,
    prev: *mut TyrusDoubleLinkedListNode,
    next: *mut TyrusDoubleLinkedListNode,
}

pub struct TyrusDoubleLinkedList {
    head: *mut TyrusDoubleLinkedListNode,
    tail: *mut TyrusDoubleLinkedListNode,
    len: usize,
    element_size: usize,
}

unsafe fn new_node(element_size: usize, value: *const u8) -> *mut TyrusDoubleLinkedListNode {
    unsafe {
        if element_size == 0 {
            allocation_failed();
        }
        let node = Box::new(TyrusDoubleLinkedListNode {
            value: vec![0u8; element_size],
            prev: std::ptr::null_mut(),
            next: std::ptr::null_mut(),
        });
        let raw = Box::into_raw(node);
        std::ptr::copy_nonoverlapping(value, (*raw).value.as_mut_ptr(), element_size);
        raw
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_new(
    element_size: usize,
) -> *mut TyrusDoubleLinkedList {
    if element_size == 0 {
        allocation_failed();
    }
    Box::into_raw(Box::new(TyrusDoubleLinkedList {
        head: std::ptr::null_mut(),
        tail: std::ptr::null_mut(),
        len: 0,
        element_size,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_from_array(
    element_size: usize,
    data: *const u8,
    length: usize,
) -> *mut TyrusDoubleLinkedList {
    unsafe {
        let list = Box::new(TyrusDoubleLinkedList {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            len: 0,
            element_size,
        });
        let raw = Box::into_raw(list);
        for index in 0..length {
            let element = data.add(index * element_size);
            tyrus_double_linked_list_push_back(raw, element);
        }
        raw
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_push_front(
    list: *mut TyrusDoubleLinkedList,
    item: *const u8,
) {
    unsafe {
        let list = &mut *list;
        let node = new_node(list.element_size, item);
        (*node).next = list.head;
        if !list.head.is_null() {
            (*list.head).prev = node;
        } else {
            list.tail = node;
        }
        list.head = node;
        list.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_push_back(
    list: *mut TyrusDoubleLinkedList,
    item: *const u8,
) {
    unsafe {
        let list = &mut *list;
        let node = new_node(list.element_size, item);
        (*node).prev = list.tail;
        if !list.tail.is_null() {
            (*list.tail).next = node;
        } else {
            list.head = node;
        }
        list.tail = node;
        list.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_pop_front(
    list: *mut TyrusDoubleLinkedList,
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
        if !list.head.is_null() {
            (*list.head).prev = std::ptr::null_mut();
        } else {
            list.tail = std::ptr::null_mut();
        }
        list.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_pop_back(
    list: *mut TyrusDoubleLinkedList,
    output: *mut u8,
) -> bool {
    unsafe {
        let list = &mut *list;
        if list.tail.is_null() {
            return false;
        }
        let node = Box::from_raw(list.tail);
        std::ptr::copy_nonoverlapping(node.value.as_ptr(), output, list.element_size);
        list.tail = node.prev;
        if !list.tail.is_null() {
            (*list.tail).next = std::ptr::null_mut();
        } else {
            list.head = std::ptr::null_mut();
        }
        list.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_front(
    list: *const TyrusDoubleLinkedList,
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
pub unsafe extern "C" fn tyrus_double_linked_list_back(
    list: *const TyrusDoubleLinkedList,
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
pub unsafe extern "C" fn tyrus_double_linked_list_len(list: *const TyrusDoubleLinkedList) -> usize {
    unsafe { (*list).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_is_empty(
    list: *const TyrusDoubleLinkedList,
) -> bool {
    unsafe { (*list).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_double_linked_list_clear(list: *mut TyrusDoubleLinkedList) {
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
pub unsafe extern "C" fn tyrus_double_linked_list_free(list: *mut TyrusDoubleLinkedList) {
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
