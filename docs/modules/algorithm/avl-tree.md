---
title: AvlTree
description: "Self-balancing AVL tree from tyrus:algorithm."
---

# AvlTree

`AvlTree<T>` is an ordered set backed by a self-balancing AVL tree. Every insertion rebalances the tree so that the two subtrees of any node differ by at most one level in height, which guarantees logarithmic operations in the worst case. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `AvlTree<T>()` | Create a new empty AVL tree |

`T` can be any integer type, any float type, `boolean`, or `string`.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `insert` | `(T) => void` | `void` | Add an element; duplicates are ignored |
| `contains` | `(T) => boolean` | `boolean` | Whether the element is in the tree |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the tree is empty |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `insert` | O(log n) worst case |
| `contains` | O(log n) worst case |
| `len` / `isEmpty` | O(1) |

The strict height invariant makes the worst case as fast as the average case: every operation is logarithmic regardless of insertion order.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/avl-tree/complete-example.md -->
