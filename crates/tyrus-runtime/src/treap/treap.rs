use std::sync::atomic::{AtomicU64, Ordering};

pub struct TyrusTreap {
    root: Option<Box<TreapNode>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
    prng: u64,
}

struct TreapNode {
    key: Box<[u8]>,
    priority: u32,
    left: Option<Box<TreapNode>>,
    right: Option<Box<TreapNode>>,
}

unsafe impl Send for TyrusTreap {}
unsafe impl Sync for TyrusTreap {}

static SEED: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);

fn splitmix64(mut state: u64) -> u64 {
    state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn next_priority(state: &mut u64) -> u32 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
}

fn rotate_right(mut node: Box<TreapNode>) -> Box<TreapNode> {
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    pivot.right = Some(node);
    pivot
}

fn rotate_left(mut node: Box<TreapNode>) -> Box<TreapNode> {
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    pivot.left = Some(node);
    pivot
}

fn treap_insert(
    node: Option<Box<TreapNode>>,
    key: &[u8],
    priority: u32,
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> (Option<Box<TreapNode>>, bool) {
    let Some(mut node) = node else {
        return (
            Some(Box::new(TreapNode {
                key: key.to_vec().into_boxed_slice(),
                priority,
                left: None,
                right: None,
            })),
            true,
        );
    };
    let inserted = {
        let cmp = compare(key, &node.key);
        if cmp < 0 {
            let (new_left, inserted) = treap_insert(node.left.take(), key, priority, compare);
            node.left = new_left;
            inserted
        } else if cmp > 0 {
            let (new_right, inserted) = treap_insert(node.right.take(), key, priority, compare);
            node.right = new_right;
            inserted
        } else {
            false
        }
    };
    if let Some(left) = &node.left
        && left.priority > node.priority
    {
        node = rotate_right(node);
    }
    if let Some(right) = &node.right
        && right.priority > node.priority
    {
        node = rotate_left(node);
    }
    (Some(node), inserted)
}

fn treap_contains(
    node: &Option<Box<TreapNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> bool {
    let Some(node) = node else {
        return false;
    };
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        treap_contains(&node.left, key, compare)
    } else if cmp > 0 {
        treap_contains(&node.right, key, compare)
    } else {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusTreap {
    let seed = SEED.fetch_add(0x9e37_79b9, Ordering::Relaxed);
    let tree = Box::new(TyrusTreap {
        root: None,
        len: 0,
        element_size,
        compare,
        prng: splitmix64(seed),
    });
    Box::into_raw(tree)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_insert(tree: *mut TyrusTreap, element: *const u8) {
    unsafe {
        let tree = &mut *tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        let priority = next_priority(&mut tree.prng);
        let (root, inserted) = treap_insert(tree.root.take(), key, priority, compare);
        tree.root = root;
        if inserted {
            tree.len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_contains(tree: *const TyrusTreap, element: *const u8) -> bool {
    unsafe {
        let tree = &*tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        treap_contains(&tree.root, key, compare)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_len(tree: *const TyrusTreap) -> usize {
    unsafe { (*tree).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_is_empty(tree: *const TyrusTreap) -> bool {
    unsafe { (*tree).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_treap_free(tree: *mut TyrusTreap) {
    unsafe {
        if !tree.is_null() {
            drop(Box::from_raw(tree));
        }
    }
}
