pub struct TyrusAvlTree {
    root: Option<Box<AvlNode>>,
    len: usize,
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
}

struct AvlNode {
    key: Box<[u8]>,
    height: i32,
    left: Option<Box<AvlNode>>,
    right: Option<Box<AvlNode>>,
}

fn height(node: &Option<Box<AvlNode>>) -> i32 {
    node.as_ref().map_or(0, |node| node.height)
}

fn update_height(node: &mut AvlNode) {
    node.height = 1 + height(&node.left).max(height(&node.right));
}

fn balance_factor(node: &AvlNode) -> i32 {
    height(&node.left) - height(&node.right)
}

fn node_balance_factor(node: &AvlNode) -> i32 {
    balance_factor(node)
}

fn rotate_right(mut node: Box<AvlNode>) -> Box<AvlNode> {
    let mut pivot = node
        .left
        .take()
        .expect("rotate_right requires a left child");
    node.left = pivot.right.take();
    update_height(&mut node);
    pivot.right = Some(node);
    update_height(&mut pivot);
    pivot
}

fn rotate_left(mut node: Box<AvlNode>) -> Box<AvlNode> {
    let mut pivot = node
        .right
        .take()
        .expect("rotate_left requires a right child");
    node.right = pivot.left.take();
    update_height(&mut node);
    pivot.left = Some(node);
    update_height(&mut pivot);
    pivot
}

fn rebalance(mut node: Box<AvlNode>) -> Box<AvlNode> {
    let balance = balance_factor(&node);
    if balance > 1 {
        if node.left.as_ref().map_or(0, |n| node_balance_factor(n)) >= 0 {
            rotate_right(node)
        } else {
            let left = node.left.take().unwrap();
            node.left = Some(rotate_left(left));
            rotate_right(node)
        }
    } else if balance < -1 {
        if node.right.as_ref().map_or(0, |n| node_balance_factor(n)) <= 0 {
            rotate_left(node)
        } else {
            let right = node.right.take().unwrap();
            node.right = Some(rotate_right(right));
            rotate_left(node)
        }
    } else {
        node
    }
}

fn avl_insert(
    node: Option<Box<AvlNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> (Option<Box<AvlNode>>, bool) {
    let Some(mut node) = node else {
        return (
            Some(Box::new(AvlNode {
                key: key.to_vec().into_boxed_slice(),
                height: 1,
                left: None,
                right: None,
            })),
            true,
        );
    };
    let inserted = {
        let cmp = compare(key, &node.key);
        if cmp < 0 {
            let (new_left, inserted) = avl_insert(node.left.take(), key, compare);
            node.left = new_left;
            inserted
        } else if cmp > 0 {
            let (new_right, inserted) = avl_insert(node.right.take(), key, compare);
            node.right = new_right;
            inserted
        } else {
            false
        }
    };
    update_height(&mut node);
    (Some(rebalance(node)), inserted)
}

fn avl_contains(
    node: &Option<Box<AvlNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> bool {
    let Some(node) = node else {
        return false;
    };
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        avl_contains(&node.left, key, compare)
    } else if cmp > 0 {
        avl_contains(&node.right, key, compare)
    } else {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_new(
    element_size: usize,
    compare: Option<unsafe extern "C" fn(*const u8, *const u8) -> i32>,
) -> *mut TyrusAvlTree {
    let tree = Box::new(TyrusAvlTree {
        root: None,
        len: 0,
        element_size,
        compare,
    });
    Box::into_raw(tree)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_insert(tree: *mut TyrusAvlTree, element: *const u8) {
    unsafe {
        let tree = &mut *tree;
        let compare_fn = tree.compare;
        let compare: &dyn Fn(&[u8], &[u8]) -> i32 = &move |a: &[u8], b: &[u8]| match compare_fn {
            Some(f) => f(a.as_ptr(), b.as_ptr()),
            None => 0,
        };
        let key = std::slice::from_raw_parts(element, tree.element_size);
        let (root, inserted) = avl_insert(tree.root.take(), key, compare);
        tree.root = root;
        if inserted {
            tree.len += 1;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_contains(
    tree: *const TyrusAvlTree,
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
        avl_contains(&tree.root, key, compare)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_len(tree: *const TyrusAvlTree) -> usize {
    unsafe { (*tree).len }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_is_empty(tree: *const TyrusAvlTree) -> bool {
    unsafe { (*tree).len == 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_avl_tree_free(tree: *mut TyrusAvlTree) {
    unsafe {
        if !tree.is_null() {
            drop(Box::from_raw(tree));
        }
    }
}
