---
title: BinaryHeap
description: "Binary max-heap priority queue from tyrus:algorithm."
---

# BinaryHeap

`BinaryHeap<T>` implements a binary **max-heap** priority queue implemented in the native runtime. Elements are stored in a contiguous array and ordered by value, with the largest element always at the top.

## Construction

| Syntax | Description |
|--------|-------------|
| `BinaryHeap<T>()` | Create a new empty heap with default capacity |

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
| `push` | O(log n) |
| `pop` | O(log n) |
| `peek` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/construction.md -->

### push

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/push.md -->

### pop

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/pop.md -->

### peek

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/peek.md -->

### len

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/len.md -->

### isEmpty

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/isempty.md -->

### clear

<!-- @include: ../../.fragments/modules/algorithm/binary-heap/clear.md -->

