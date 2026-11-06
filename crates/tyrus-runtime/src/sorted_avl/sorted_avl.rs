//! Shared self-balancing AVL tree core for `SortedSet<T>` and `SortedMap<K, V>`.
//!
//! Nodes carry an opaque key byte string plus an optional value byte string,
//! `SortedSet` uses `value: None` and `SortedMap` stores one `value_size`-wide
//! blob per key. The caller owns the comparison function (an `extern "C"`
//! `fn(*const u8, *const u8) -> i32` supplied by generated code) and the
//! element/value byte widths.

pub struct AvlNode {
    pub key: Box<[u8]>,
    pub value: Option<Box<[u8]>>,
    pub height: i32,
    pub left: Option<Box<AvlNode>>,
    pub right: Option<Box<AvlNode>>,
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

pub fn insert(
    node: Option<Box<AvlNode>>,
    key: &[u8],
    value: Option<Box<[u8]>>,
    overwrite: bool,
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> (Option<Box<AvlNode>>, bool) {
    let Some(mut node) = node else {
        return (
            Some(Box::new(AvlNode {
                key: key.to_vec().into_boxed_slice(),
                value,
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
            let (new_left, inserted) = insert(node.left.take(), key, value, overwrite, compare);
            node.left = new_left;
            inserted
        } else if cmp > 0 {
            let (new_right, inserted) = insert(node.right.take(), key, value, overwrite, compare);
            node.right = new_right;
            inserted
        } else if overwrite {
            node.value = value;
            false
        } else {
            false
        }
    };
    update_height(&mut node);
    (Some(rebalance(node)), inserted)
}

fn take_min(mut node: Box<AvlNode>) -> (Option<Box<AvlNode>>, Box<AvlNode>) {
    if let Some(left) = node.left.take() {
        let (new_left, min) = take_min(left);
        node.left = new_left;
        update_height(&mut node);
        (Some(rebalance(node)), min)
    } else {
        (node.right.take(), node)
    }
}

pub struct RemoveResult {
    pub root: Option<Box<AvlNode>>,
    pub deleted: bool,
    pub removed: Option<Box<[u8]>>,
}

impl RemoveResult {
    fn new(root: Option<Box<AvlNode>>, deleted: bool, removed: Option<Box<[u8]>>) -> Self {
        Self {
            root,
            deleted,
            removed,
        }
    }
}

pub fn delete(
    node: Option<Box<AvlNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> RemoveResult {
    let Some(mut node) = node else {
        return RemoveResult::new(None, false, None);
    };
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        let result = delete(node.left.take(), key, compare);
        node.left = result.root;
        if result.deleted {
            update_height(&mut node);
            return RemoveResult::new(Some(rebalance(node)), true, result.removed);
        }
        return RemoveResult::new(Some(node), false, None);
    }
    if cmp > 0 {
        let result = delete(node.right.take(), key, compare);
        node.right = result.root;
        if result.deleted {
            update_height(&mut node);
            return RemoveResult::new(Some(rebalance(node)), true, result.removed);
        }
        return RemoveResult::new(Some(node), false, None);
    }
    let removed = node.value.take();
    match (node.left.take(), node.right.take()) {
        (None, None) => RemoveResult::new(None, true, removed),
        (Some(left), None) => RemoveResult::new(Some(left), true, removed),
        (None, Some(right)) => RemoveResult::new(Some(right), true, removed),
        (Some(left), Some(right)) => {
            let (right_remaining, mut successor) = take_min(right);
            successor.left = Some(left);
            successor.right = right_remaining;
            update_height(&mut successor);
            RemoveResult::new(Some(rebalance(successor)), true, removed)
        }
    }
}

pub fn contains(
    node: &Option<Box<AvlNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> bool {
    let Some(node) = node else {
        return false;
    };
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        contains(&node.left, key, compare)
    } else if cmp > 0 {
        contains(&node.right, key, compare)
    } else {
        true
    }
}

pub fn find<'a>(
    node: &'a Option<Box<AvlNode>>,
    key: &[u8],
    compare: &dyn Fn(&[u8], &[u8]) -> i32,
) -> Option<&'a [u8]> {
    let node = node.as_ref()?;
    let cmp = compare(key, &node.key);
    if cmp < 0 {
        find(&node.left, key, compare)
    } else if cmp > 0 {
        find(&node.right, key, compare)
    } else {
        node.value.as_deref()
    }
}

pub fn min_key(node: &Option<Box<AvlNode>>) -> Option<&[u8]> {
    let mut node = node.as_ref()?;
    while let Some(left) = node.left.as_ref() {
        node = left;
    }
    Some(&node.key)
}

pub fn max_key(node: &Option<Box<AvlNode>>) -> Option<&[u8]> {
    let mut node = node.as_ref()?;
    while let Some(right) = node.right.as_ref() {
        node = right;
    }
    Some(&node.key)
}
