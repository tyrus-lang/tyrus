use crate::tyrus_panic;

fn out_of_bounds_index() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: SegmentTree index is out of bounds".as_ptr()) }
}

fn out_of_bounds_range() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: SegmentTree range is out of bounds".as_ptr()) }
}

pub struct TyrusSegmentTree {
    size: usize,
    tree: Vec<i32>,
}

fn build(tree: &mut [i32], data: *const i32, node: usize, l: usize, r: usize) {
    if r - l == 1 {
        tree[node] = unsafe { *data.add(l) };
    } else {
        let mid = usize::midpoint(l, r);
        build(tree, data, 2 * node + 1, l, mid);
        build(tree, data, 2 * node + 2, mid, r);
        tree[node] = tree[2 * node + 1] + tree[2 * node + 2];
    }
}

fn update_rec(tree: &mut [i32], node: usize, l: usize, r: usize, index: usize, value: i32) {
    if r - l == 1 {
        tree[node] = value;
        return;
    }
    let mid = usize::midpoint(l, r);
    if index < mid {
        update_rec(tree, 2 * node + 1, l, mid, index, value);
    } else {
        update_rec(tree, 2 * node + 2, mid, r, index, value);
    }
    tree[node] = tree[2 * node + 1] + tree[2 * node + 2];
}

fn query_rec(tree: &[i32], node: usize, l: usize, r: usize, from: usize, to: usize) -> i32 {
    if from <= l && r <= to {
        return tree[node];
    }
    if to <= l || r <= from {
        return 0;
    }
    let mid = usize::midpoint(l, r);
    query_rec(tree, 2 * node + 1, l, mid, from, to)
        + query_rec(tree, 2 * node + 2, mid, r, from, to)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_new(
    data: *const i32,
    length: usize,
) -> *mut TyrusSegmentTree {
    let capacity = match length.checked_mul(4).map(|capacity| capacity.max(1)) {
        Some(capacity) => capacity,
        None => out_of_bounds_index(),
    };
    let mut tree = vec![0i32; capacity];
    if length > 0 {
        build(&mut tree, data, 0, 0, length);
    }
    let st = Box::new(TyrusSegmentTree { size: length, tree });
    Box::into_raw(st)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_update(
    st: *mut TyrusSegmentTree,
    index: usize,
    value: i32,
) {
    unsafe {
        let st = &mut *st;
        if index >= st.size {
            out_of_bounds_index();
        }
        update_rec(&mut st.tree, 0, 0, st.size, index, value);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_query(
    st: *const TyrusSegmentTree,
    from: usize,
    to: usize,
) -> i32 {
    unsafe {
        let st = &*st;
        if from > to || to > st.size {
            out_of_bounds_range();
        }
        query_rec(&st.tree, 0, 0, st.size, from, to)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_len(st: *const TyrusSegmentTree) -> usize {
    unsafe { (*st).size }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_is_empty(st: *const TyrusSegmentTree) -> bool {
    unsafe { (*st).size == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_free(st: *mut TyrusSegmentTree) {
    unsafe {
        if !st.is_null() {
            drop(Box::from_raw(st));
        }
    }
}
