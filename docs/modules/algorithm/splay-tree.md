---
title: SplayTree
description: "Self-adjusting splay tree from tyrus:algorithm."
---

# SplayTree

`SplayTree<T>` is an ordered set backed by a self-adjusting splay tree. Every access moves the element to the root through a sequence of rotations, so recently accessed elements become cheaper to reach again. Operations are amortized logarithmic. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `SplayTree<T>()` | Create a new empty splay tree |

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
| `insert` | O(log n) amortized |
| `contains` | O(log n) amortized |
| `len` / `isEmpty` | O(1) |

A single operation can touch every node, but a sequence of m operations on a tree of n elements always runs in O(m log n): the amortized cost of each operation is O(log n), and access patterns with temporal locality are even faster in practice.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/splay-tree/complete-example.md -->
