use crate::tyrus_panic;

fn out_of_bounds() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: FenwickTree index is out of bounds".as_ptr()) }
}

pub struct TyrusFenwickTree {
    size: usize,
    tree: Vec<i32>,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_new(size: usize) -> *mut TyrusFenwickTree {
    let capacity = match size.checked_add(1) {
        Some(capacity) => capacity,
        None => out_of_bounds(),
    };
    let ft = Box::new(TyrusFenwickTree {
        size,
        tree: vec![0i32; capacity],
    });
    Box::into_raw(ft)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_add(
    ft: *mut TyrusFenwickTree,
    index: usize,
    delta: i32,
) {
    unsafe {
        let ft = &mut *ft;
        if index >= ft.size {
            out_of_bounds();
        }
        let mut idx = index + 1;
        while idx < ft.tree.len() {
            ft.tree[idx] += delta;
            idx += idx & (!idx + 1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_sum(ft: *const TyrusFenwickTree, index: usize) -> i32 {
    unsafe {
        let ft = &*ft;
        if index > ft.size {
            out_of_bounds();
        }
        let mut idx = index;
        let mut sum = 0i32;
        while idx > 0 {
            sum += ft.tree[idx];
            idx -= idx & (!idx + 1);
        }
        sum
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_len(ft: *const TyrusFenwickTree) -> usize {
    unsafe { (*ft).size }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_is_empty(ft: *const TyrusFenwickTree) -> bool {
    unsafe { (*ft).size == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_fenwick_tree_free(ft: *mut TyrusFenwickTree) {
    unsafe {
        if !ft.is_null() {
            drop(Box::from_raw(ft));
        }
    }
}
