---
title: SortedSet
description: "Ordered set data structure from tyrus:algorithm."
---

# SortedSet

`SortedSet<T>` is an ordered set backed by a balanced binary search tree. It keeps every element in sorted order and ignores duplicate inserts. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `SortedSet<T>()` | Create a new empty ordered set |

`T` can be any integer type, any float type, `boolean`, or `string`.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `insert` | `(T) => boolean` | `boolean` | Add an element; returns `true` when it was not present |
| `remove` | `(T) => boolean` | `boolean` | Remove an element; returns `true` when it was present |
| `contains` | `(T) => boolean` | `boolean` | Whether the element is in the set |
| `min` | `() => T?` | `T?` | Smallest element, or no value when empty |
| `max` | `() => T?` | `T?` | Largest element, or no value when empty |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the set is empty |
| `clear` | `() => void` | `void` | Remove all elements |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `insert` | O(log n) |
| `remove` | O(log n) |
| `contains` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

The tree stays balanced, so every lookup, insertion and deletion is logarithmic regardless of insertion order.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/sorted-set/complete-example.md -->

### String ordering

<!-- @include: ../../.fragments/modules/algorithm/sorted-set/string-ordering.md -->
