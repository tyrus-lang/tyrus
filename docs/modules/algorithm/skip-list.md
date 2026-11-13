---
title: SkipList
description: "Randomized skip list (ordered set) from tyrus:algorithm."
---

# SkipList

`SkipList<T>` is an ordered set backed by a skip list: a linked list of nodes threaded with randomized express lanes. It keeps every element in sorted order and ignores duplicate inserts. Levels are drawn at random up to 16, so each search, insertion and removal is O(log n) on average, and the lane layout requires no rebalancing. The native handle is released automatically when the binding goes out of scope.

## Construction

| Syntax | Description |
|--------|-------------|
| `SkipList<T>()` | Create a new empty skip list |

`T` can be any integer type, any float type, `boolean`, or `string`.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `insert` | `(T) => boolean` | `boolean` | Add an element; returns `true` when it was not present |
| `remove` | `(T) => boolean` | `boolean` | Remove an element; returns `true` when it was present |
| `contains` | `(T) => boolean` | `boolean` | Whether the element is in the list |
| `min` | `() => T?` | `T?` | Smallest element, or no value when empty |
| `max` | `() => T?` | `T?` | Largest element, or no value when empty |
| `len` | `() => usize` | `usize` | Number of elements |
| `isEmpty` | `() => boolean` | `boolean` | Whether the list is empty |
| `clear` | `() => void` | `void` | Remove all elements |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `insert` | O(log n) expected |
| `remove` | O(log n) expected |
| `contains` | O(log n) expected |
| `min` | O(1) |
| `max` | O(n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

The expected logarithmic bounds hold for any insertion order, because the express lanes are built from randomized levels rather than from the history of updates; the two lane extremes are found by walking the bottom lane, so `min` reads the first node and `max` walks to the last one.

## Examples

### Complete example

<!-- @include: ../../.fragments/modules/algorithm/skip-list/complete-example.md -->

### String ordering

<!-- @include: ../../.fragments/modules/algorithm/skip-list/string-ordering.md -->
