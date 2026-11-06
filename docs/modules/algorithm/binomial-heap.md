---
title: BinomialHeap
description: "Binomial max-heap priority queue from tyrus:algorithm."
---

# BinomialHeap

`BinomialHeap<T>` implements a **max-heap** priority queue as a forest of binomial trees implemented in the native runtime. Unlike a binary heap, which keeps a single contiguous array, the binomial heap stores its elements in a collection of binomial trees whose sizes are the binary digits of the element count, so merging two heaps costs O(log n) instead of rebuilding the whole structure.

## Construction

| Syntax | Description |
|--------|-------------|
| `BinomialHeap<T>()` | Create a new empty heap |

The type parameter `T` determines the element type. `T` must be an ordered numeric type (`i8`–`i128`, `u8`–`u128`, `f32`, `f64`, or `f128`).

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `push` | `(T) => void` | `void` | Insert an element |
| `pop` | `() => T?` | `T?` | Remove and return the largest element, or empty if empty |
| `peek` | `() => T?` | `T?` | Return the largest element without removing it, or empty if empty |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the heap has no elements |
| `clear` | `() => void` | `void` | Remove all elements |

The native handle is released automatically when the binding goes out of scope.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `push` | Amortized O(1), worst-case O(log n) |
| `pop` | O(log n) |
| `peek` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

`peek()` walks the root list, so it is O(log n) rather than the O(1) of a binary heap. `push` takes amortized O(1) because most inserts only append a new tree to the root list; a `pop` that removes it rebuilds the heap in O(log n).

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/construction.md -->

### push

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/push.md -->

### pop

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/pop.md -->

### peek

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/peek.md -->

### len

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/len.md -->

### isEmpty

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/isempty.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/binomial-heap/clear.md -->
