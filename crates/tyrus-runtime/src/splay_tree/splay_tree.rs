pub struct TyrusSplayTree {
    root: Option<Box<SplayNode>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

struct SplayNode {
    key: Box<[u8]>,
    left: Option<Box<SplayNode>>,
    right: Option<Box<SplayNode>>,
}

unsafe impl Send for TyrusSplayTree {}
unsafe impl Sync for TyrusSplayTree {}

fn rotate_right(mut node: Box<SplayNode>) -> Box<SplayNode> {
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    pivot.right = Some(node);
    pivot
}

fn rotate_left(mut node: Box<SplayNode>) -> Box<SplayNode> {
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    pivot.left = Some(node);
    pivot
}

fn splay(
    mut root: Box<SplayNode>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> Box<SplayNode> {
    let cmp = compare(key, &root.key);
    if cmp == 0 {
        return root;
    }
    if cmp < 0 {
        let Some(mut left) = root.left.take() else {
            return root;
        };
        let lcmp = compare(key, &left.key);
        if lcmp < 0 {
            let ll = left.left.take();
            let new_ll = ll.map(|ll| splay(ll, key, compare));
            left.left = new_ll;
            root.left = Some(left);
            let first = rotate_right(root);
            if first.left.is_some() {
                rotate_right(first)
            } else {
                first
            }
        } else if lcmp > 0 {
            let lr = left.right.take();
            let new_lr = lr.map(|lr| splay(lr, key, compare));
            left.right = new_lr;
            if left.right.is_some() {
                left = rotate_left(left);
            }
            root.left = Some(left);
            rotate_right(root)
        } else {
            root.left = left.right.take();
            left.right = Some(root);
            left
        }
    } else {
        let Some(mut right) = root.right.take() else {
            return root;
        };
        let rcmp = compare(key, &right.key);
        if rcmp > 0 {
            let rr = right.right.take();
            let new_rr = rr.map(|rr| splay(rr, key, compare));
            right.right = new_rr;
            root.right = Some(right);
            let first = rotate_left(root);
            if first.right.is_some() {
                rotate_left(first)
            } else {
                first
            }
        } else if rcmp < 0 {
            let rl = right.left.take();
            let new_rl = rl.map(|rl| splay(rl, key, compare));
            right.left = new_rl;
            if right.left.is_some() {
                right = rotate_right(right);
            }
            root.right = Some(right);
            rotate_left(root)
        } else {
            root.right = right.left.take();
            right.left = Some(root);
            right
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusSplayTree {
    let tree = Box::new(TyrusSplayTree {
        root: None,
        len: 0,
        element_size,
        compare,
    });
    Box::into_raw(tree)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_insert(tree: *mut TyrusSplayTree, element: *const u8) {
    unsafe {
        let tree = &mut *tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        let splayed = match tree.root.take() {
            Some(root) => splay(root, key, compare),
            None => {
                tree.root = Some(Box::new(SplayNode {
                    key: key.to_vec().into_boxed_slice(),
                    left: None,
                    right: None,
                }));
                tree.len += 1;
                return;
            }
        };
        if compare(key, &splayed.key) == 0 {
            tree.root = Some(splayed);
            return;
        }
        let key_slice = key.to_vec().into_boxed_slice();
        let mut new_node = Box::new(SplayNode {
            key: key_slice,
            left: None,
            right: None,
        });
        if compare(key, &splayed.key) < 0 {
            let mut splayed = splayed;
            new_node.left = splayed.left.take();
            new_node.right = Some(splayed);
        } else {
            let mut splayed = splayed;
            new_node.right = splayed.right.take();
            new_node.left = Some(splayed);
        }
        tree.root = Some(new_node);
        tree.len += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_contains(
    tree: *mut TyrusSplayTree,
    element: *const u8,
) -> bool {
    unsafe {
        let tree = &mut *tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        let Some(splayed) = tree.root.take().map(|root| splay(root, key, compare)) else {
            return false;
        };
        let found = compare(key, &splayed.key) == 0;
        tree.root = Some(splayed);
        found
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_len(tree: *const TyrusSplayTree) -> usize {
    unsafe { (*tree).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_is_empty(tree: *const TyrusSplayTree) -> bool {
    unsafe { (*tree).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_splay_tree_free(tree: *mut TyrusSplayTree) {
    unsafe {
        if !tree.is_null() {
            drop(Box::from_raw(tree));
        }
    }
}
