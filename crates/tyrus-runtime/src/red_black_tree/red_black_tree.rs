const RED: bool = false;
const BLACK: bool = true;

pub struct TyrusRedBlackTree {
    root: Option<Box<RbNode>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

struct RbNode {
    key: Box<[u8]>,
    color: bool,
    left: Option<Box<RbNode>>,
    right: Option<Box<RbNode>>,
}

unsafe impl Send for TyrusRedBlackTree {}
unsafe impl Sync for TyrusRedBlackTree {}

fn color_of(node: &Option<Box<RbNode>>) -> bool {
    node.as_ref().map_or(BLACK, |node| node.color)
}

fn rotate_left(mut node: Box<RbNode>) -> Box<RbNode> {
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    pivot.left = Some(node);
    pivot
}

fn rotate_right(mut node: Box<RbNode>) -> Box<RbNode> {
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    pivot.right = Some(node);
    pivot
}

fn rb_insert(
    node: Option<Box<RbNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> (Option<Box<RbNode>>, bool) {
    let Some(mut node) = node else {
        return (
            Some(Box::new(RbNode {
                key: key.to_vec().into_boxed_slice(),
                color: RED,
                left: None,
                right: None,
            })),
            true,
        );
    };
    let inserted = {
        if color_of(&node.left) == RED && color_of(&node.right) == RED {
            node.color = RED;
            if let Some(left) = node.left.as_mut() {
                left.color = BLACK;
            }
            if let Some(right) = node.right.as_mut() {
                right.color = BLACK;
            }
        }
        let cmp = compare(key, &node.key);
        if cmp < 0 {
            let (new_left, inserted) = rb_insert(node.left.take(), key, compare);
            node.left = new_left;
            inserted
        } else if cmp > 0 {
            let (new_right, inserted) = rb_insert(node.right.take(), key, compare);
            node.right = new_right;
            inserted
        } else {
            false
        }
    };
    if color_of(&node.right) == RED && color_of(&node.left) != RED {
        node = rotate_left(node);
    }
    if color_of(&node.left) == RED
        && node
            .left
            .as_ref()
            .is_some_and(|left| color_of(&left.left) == RED)
    {
        node = rotate_right(node);
    }
    (Some(node), inserted)
}

fn rb_contains(
    node: &Option<Box<RbNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> bool {
    let Some(node) = node else {
        return false;
    };
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        rb_contains(&node.left, key, compare)
    } else if cmp > 0 {
        rb_contains(&node.right, key, compare)
    } else {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusRedBlackTree {
    let tree = Box::new(TyrusRedBlackTree {
        root: None,
        len: 0,
        element_size,
        compare,
    });
    Box::into_raw(tree)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_insert(
    tree: *mut TyrusRedBlackTree,
    element: *const u8,
) {
    unsafe {
        let tree = &mut *tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        let (root, inserted) = rb_insert(tree.root.take(), key, compare);
        let mut root = root;
        if let Some(root) = root.as_mut() {
            root.color = BLACK;
        }
        tree.root = root;
        if inserted {
            tree.len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_contains(
    tree: *const TyrusRedBlackTree,
    element: *const u8,
) -> bool {
    unsafe {
        let tree = &*tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        rb_contains(&tree.root, key, compare)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_len(tree: *const TyrusRedBlackTree) -> usize {
    unsafe { (*tree).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_is_empty(tree: *const TyrusRedBlackTree) -> bool {
    unsafe { (*tree).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_red_black_tree_free(tree: *mut TyrusRedBlackTree) {
    unsafe {
        if !tree.is_null() {
            drop(Box::from_raw(tree));
        }
    }
}
