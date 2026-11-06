use crate::TyrusArray;
use std::ffi::c_void;

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
}

fn allocation_failed() -> ! {
    unsafe { crate::tyrus_panic(c"tyrus runtime error: BinomialHeap allocation failed".as_ptr()) }
}

pub struct TyrusBinomialHeap {
    roots: Vec<Option<Box<BinomialNode>>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

struct BinomialNode {
    key: Box<[u8]>,
    degree: usize,
    children: Vec<BinomialNode>,
}

fn link(
    a: Box<BinomialNode>,
    b: Box<BinomialNode>,
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> Box<BinomialNode> {
    let (mut parent, child) = if compare(&a.key, &b.key) >= 0 {
        (a, b)
    } else {
        (b, a)
    };
    parent.children.push(*child);
    parent.degree += 1;
    parent
}

fn meld(
    a: &mut Vec<Option<Box<BinomialNode>>>,
    b: &mut Vec<Option<Box<BinomialNode>>>,
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> Vec<Option<Box<BinomialNode>>> {
    let n = a.len().max(b.len());
    a.resize_with(n, || None);
    b.resize_with(n, || None);
    let mut out: Vec<Option<Box<BinomialNode>>> = Vec::with_capacity(n + 1);
    let mut carry: Option<Box<BinomialNode>> = None;
    for i in 0..n {
        let mut group: Vec<Box<BinomialNode>> = Vec::with_capacity(3);
        if let Some(tree) = a[i].take() {
            group.push(tree);
        }
        if let Some(tree) = b[i].take() {
            group.push(tree);
        }
        if let Some(tree) = carry.take() {
            group.push(tree);
        }
        match group.len() {
            0 => out.push(None),
            1 => out.push(group.pop()),
            2 => carry = Some(link(group.pop().unwrap(), group.pop().unwrap(), compare)),
            _ => {
                out.push(group.pop());
                carry = Some(link(group.pop().unwrap(), group.pop().unwrap(), compare));
            }
        }
    }
    if let Some(carry) = carry {
        out.push(Some(carry));
    }
    out
}

fn max_root_index(
    roots: &[Option<Box<BinomialNode>>],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, tree) in roots.iter().enumerate() {
        if let Some(node) = tree {
            match best {
                None => best = Some(i),
                Some(best_index) => {
                    if compare(&node.key, &roots[best_index].as_ref().unwrap().key) > 0 {
                        best = Some(i);
                    }
                }
            }
        }
    }
    best
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusBinomialHeap {
    let heap = Box::new(TyrusBinomialHeap {
        roots: Vec::new(),
        len: 0,
        element_size,
        compare,
    });
    Box::into_raw(heap)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_push(
    heap: *mut TyrusBinomialHeap,
    element: *const u8,
) {
    unsafe {
        let heap = &mut *heap;
        let compare_fn = heap.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, heap.element_size)
            .to_vec()
            .into_boxed_slice();
        let single = Box::new(BinomialNode {
            key,
            degree: 0,
            children: Vec::new(),
        });
        let mut forest = vec![Some(single)];
        heap.roots = meld(&mut heap.roots, &mut forest, compare);
        heap.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_pop(
    heap: *mut TyrusBinomialHeap,
    output: *mut u8,
) -> bool {
    unsafe {
        let heap = &mut *heap;
        let compare_fn = heap.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let Some(index) = max_root_index(&heap.roots, compare) else {
            return false;
        };
        let max = heap.roots[index].take().unwrap();
        std::ptr::copy_nonoverlapping(max.key.as_ptr(), output, heap.element_size);
        let mut forest: Vec<Option<Box<BinomialNode>>> = Vec::with_capacity(max.degree);
        for child in max.children {
            forest.push(Some(Box::new(child)));
        }
        heap.roots = meld(&mut heap.roots, &mut forest, compare);
        heap.len -= 1;
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_peek(
    heap: *const TyrusBinomialHeap,
    output: *mut u8,
) -> bool {
    unsafe {
        let heap = &*heap;
        let compare_fn = heap.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let Some(index) = max_root_index(&heap.roots, compare) else {
            return false;
        };
        std::ptr::copy_nonoverlapping(
            heap.roots[index].as_ref().unwrap().key.as_ptr(),
            output,
            heap.element_size,
        );
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_len(heap: *const TyrusBinomialHeap) -> usize {
    unsafe { (*heap).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_is_empty(heap: *const TyrusBinomialHeap) -> bool {
    unsafe { (*heap).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_clear(heap: *mut TyrusBinomialHeap) {
    unsafe {
        let heap = &mut *heap;
        heap.roots.clear();
        heap.len = 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_values(
    heap: *const TyrusBinomialHeap,
    output: *mut TyrusArray,
) {
    unsafe {
        let Some(output) = output.as_mut() else {
            return;
        };
        let Some(heap) = heap.as_ref() else {
            *output = TyrusArray {
                data: std::ptr::null_mut(),
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
            std::ptr::null_mut()
        } else {
            malloc(bytes) as *mut u8
        };
        if bytes != 0 && data.is_null() {
            allocation_failed();
        }
        if bytes != 0 {
            let mut offset = 0usize;
            for root in heap.roots.iter().flatten() {
                let mut stack: Vec<&BinomialNode> = vec![root];
                while let Some(node) = stack.pop() {
                    std::ptr::copy_nonoverlapping(
                        node.key.as_ptr(),
                        data.add(offset),
                        heap.element_size,
                    );
                    offset += heap.element_size;
                    for child in &node.children {
                        stack.push(child);
                    }
                }
            }
        }
        *output = TyrusArray {
            data,
            length: heap.len,
            capacity: heap.len,
        };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_binomial_heap_free(heap: *mut TyrusBinomialHeap) {
    unsafe {
        if !heap.is_null() {
            drop(Box::from_raw(heap));
        }
    }
}
