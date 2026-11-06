---
title: Treap
description: "Randomized treap from tyrus:algorithm."
---

# Treap

`Treap<T>` is an ordered set backed by a randomized treap: a binary search tree where each node also carries a random priority and the tree is heap-ordered by that priority. The randomness makes the tree balanced on expectation for any insertion sequence. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `Treap<T>()` | Create a new empty treap |

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
| `insert` | O(log n) expected |
| `contains` | O(log n) expected |
| `len` / `isEmpty` | O(1) |

The random priorities make the tree balanced on expectation, so every operation is logarithmic for any input, including pre-sorted data.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/treap/complete-example.md -->
