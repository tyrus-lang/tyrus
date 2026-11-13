use crate::tyrus_panic;

fn out_of_bounds_range() -> ! {
    unsafe { tyrus_panic(c"tyrus runtime error: SegmentTreeLazy range is out of bounds".as_ptr()) }
}

pub struct TyrusSegmentTreeLazy {
    size: usize,
    tree: Vec<i32>,
    lazy: Vec<i32>,
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

fn push(tree: &mut [i32], lazy: &mut [i32], node: usize, l: usize, r: usize) {
    if lazy[node] != 0 {
        tree[node] += (r - l) as i32 * lazy[node];
        if r - l > 1 {
            lazy[2 * node + 1] += lazy[node];
            lazy[2 * node + 2] += lazy[node];
        }
        lazy[node] = 0;
    }
}

fn update_range_rec(
    st: &mut TyrusSegmentTreeLazy,
    node: usize,
    l: usize,
    r: usize,
    from: usize,
    to: usize,
    delta: i32,
) {
    push(&mut st.tree, &mut st.lazy, node, l, r);
    if from <= l && r <= to {
        st.lazy[node] += delta;
        push(&mut st.tree, &mut st.lazy, node, l, r);
        return;
    }
    if to <= l || r <= from {
        return;
    }
    let mid = usize::midpoint(l, r);
    update_range_rec(st, 2 * node + 1, l, mid, from, to, delta);
    update_range_rec(st, 2 * node + 2, mid, r, from, to, delta);
    st.tree[node] = st.tree[2 * node + 1] + st.tree[2 * node + 2];
}

fn query_rec(
    st: &mut TyrusSegmentTreeLazy,
    node: usize,
    l: usize,
    r: usize,
    from: usize,
    to: usize,
) -> i32 {
    push(&mut st.tree, &mut st.lazy, node, l, r);
    if from <= l && r <= to {
        return st.tree[node];
    }
    if to <= l || r <= from {
        return 0;
    }
    let mid = usize::midpoint(l, r);
    query_rec(st, 2 * node + 1, l, mid, from, to) + query_rec(st, 2 * node + 2, mid, r, from, to)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_new(
    data: *const i32,
    length: usize,
) -> *mut TyrusSegmentTreeLazy {
    let capacity = match length.checked_mul(4).map(|capacity| capacity.max(1)) {
        Some(capacity) => capacity,
        None => out_of_bounds_range(),
    };
    let mut tree = vec![0i32; capacity];
    if length > 0 {
        build(&mut tree, data, 0, 0, length);
    }
    let st = Box::new(TyrusSegmentTreeLazy {
        size: length,
        tree,
        lazy: vec![0i32; capacity],
    });
    Box::into_raw(st)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_update_range(
    st: *mut TyrusSegmentTreeLazy,
    from: usize,
    to: usize,
    delta: i32,
) {
    unsafe {
        let st = &mut *st;
        if from > to || to > st.size {
            out_of_bounds_range();
        }
        let size = st.size;
        update_range_rec(st, 0, 0, size, from, to, delta);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_query(
    st: *mut TyrusSegmentTreeLazy,
    from: usize,
    to: usize,
) -> i32 {
    unsafe {
        let st = &mut *st;
        if from > to || to > st.size {
            out_of_bounds_range();
        }
        let size = st.size;
        query_rec(st, 0, 0, size, from, to)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_len(st: *const TyrusSegmentTreeLazy) -> usize {
    unsafe { (*st).size }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_is_empty(st: *const TyrusSegmentTreeLazy) -> bool {
    unsafe { (*st).size == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_segment_tree_lazy_free(st: *mut TyrusSegmentTreeLazy) {
    unsafe {
        if !st.is_null() {
            drop(Box::from_raw(st));
        }
    }
}
