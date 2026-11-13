use crate::TyrusArray;
use crate::tyrus_panic;
use std::ffi::c_void;
use std::ptr;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

fn allocation_failed() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: BinaryHeap allocation failed".as_ptr()) }
}

pub struct TyrusBinaryHeap {
    data: *mut u8,
    len: usize,
    capacity: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

impl TyrusBinaryHeap {
    unsafe fn grow(&mut self) {
        unsafe {
            let new_cap = if self.capacity == 0 {
                4
            } else {
                self.capacity
                    .checked_mul(2)
                    .unwrap_or_else(|| allocation_failed())
            };
            let new_size = new_cap
                .checked_mul(self.element_size)
                .unwrap_or_else(|| allocation_failed());
            let new_data = malloc(new_size) as *mut u8;
            if new_data.is_null() {
                allocation_failed();
            }
            if !self.data.is_null() && self.len > 0 {
                std::ptr::copy_nonoverlapping(self.data, new_data, self.len * self.element_size);
                free(self.data as *mut c_void);
            }
            self.data = new_data;
            self.capacity = new_cap;
        }
    }

    unsafe fn sift_up(&mut self, mut index: usize) {
        unsafe {
            let compare = match self.compare {
                Some(f) => f,
                None => return,
            };
            while index > 0 {
                let parent = (index - 1) / 2;
                let elem_size = self.element_size;
                let parent_ptr = self.data.add(parent * elem_size);
                let child_ptr = self.data.add(index * elem_size);
                if compare(parent_ptr as *const u8, child_ptr as *const u8) >= 0 {
                    break;
                }
                for i in 0..elem_size {
                    let tmp = *parent_ptr.add(i);
                    *parent_ptr.add(i) = *child_ptr.add(i);
                    *child_ptr.add(i) = tmp;
                }
                index = parent;
            }
        }
    }

    unsafe fn sift_down(&mut self, mut index: usize) {
        unsafe {
            let compare = match self.compare {
                Some(f) => f,
                None => return,
            };
            let len = self.len;
            loop {
                let left = 2 * index + 1;
                let right = 2 * index + 2;
                let mut largest = index;
                let elem_size = self.element_size;

                if left < len {
                    let largest_ptr = self.data.add(largest * elem_size);
                    let left_ptr = self.data.add(left * elem_size);
                    if compare(largest_ptr as *const u8, left_ptr as *const u8) < 0 {
                        largest = left;
                    }
                }
                if right < len {
                    let largest_ptr = self.data.add(largest * elem_size);
                    let right_ptr = self.data.add(right * elem_size);
                    if compare(largest_ptr as *const u8, right_ptr as *const u8) < 0 {
                        largest = right;
                    }
                }
                if largest == index {
                    break;
                }
                let index_ptr = self.data.add(index * elem_size);
                let largest_ptr = self.data.add(largest * elem_size);
                for i in 0..elem_size {
                    let tmp = *index_ptr.add(i);
                    *index_ptr.add(i) = *largest_ptr.add(i);
                    *largest_ptr.add(i) = tmp;
                }
                index = largest;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusBinaryHeap {
    let heap = Box::new(TyrusBinaryHeap {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
        element_size,
        compare,
    });
    Box::into_raw(heap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_len(heap: *const TyrusBinaryHeap) -> usize {
    unsafe { (*heap).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_is_empty(heap: *const TyrusBinaryHeap) -> bool {
    unsafe { (*heap).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_push(heap: *mut TyrusBinaryHeap, element: *const u8) {
    unsafe {
        let heap = &mut *heap;
        if heap.len == heap.capacity {
            heap.grow();
        }
        let elem_size = heap.element_size;
        let slot = heap.data.add(heap.len * elem_size);
        std::ptr::copy_nonoverlapping(element, slot, elem_size);
        heap.len += 1;
        heap.sift_up(heap.len - 1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_pop(
    heap: *mut TyrusBinaryHeap,
    output: *mut u8,
) -> bool {
    unsafe {
        let heap = &mut *heap;
        if heap.len == 0 {
            return false;
        }
        let elem_size = heap.element_size;
        std::ptr::copy_nonoverlapping(heap.data, output, elem_size);
        heap.len -= 1;
        if heap.len > 0 {
            std::ptr::copy_nonoverlapping(
                heap.data.add(heap.len * elem_size),
                heap.data,
                elem_size,
            );
            heap.sift_down(0);
        }
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_peek(
    heap: *const TyrusBinaryHeap,
    output: *mut u8,
) -> bool {
    unsafe {
        let heap = &*heap;
        if heap.len == 0 {
            return false;
        }
        std::ptr::copy_nonoverlapping(heap.data, output, heap.element_size);
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_clear(heap: *mut TyrusBinaryHeap) {
    unsafe {
        (*heap).len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_values(
    heap: *const TyrusBinaryHeap,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(heap) = heap.as_ref() else {
            *output = TyrusArray {
                data: ptr::null_mut(),
                length: 0,
                capacity: 0,
            };
            return;
        };
        let bytes = heap
            .len
            .checked_mul(heap.element_size)
            .unwrap_or_else(|| allocation_failed());
        let data = if bytes == 0 {
            ptr::null_mut()
        } else {
            malloc(bytes).cast::<u8>()
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }
        if bytes != 0 {
            ptr::copy_nonoverlapping(heap.data, data, bytes);
        }
        *output = TyrusArray {
            data,
            length: heap.len,
            capacity: heap.len,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binary_heap_free(heap: *mut TyrusBinaryHeap) {
    unsafe {
        if !heap.is_null() {
            let heap = Box::from_raw(heap);
            if !heap.data.is_null() {
                free(heap.data as *mut c_void);
            }
        }
    }
}
