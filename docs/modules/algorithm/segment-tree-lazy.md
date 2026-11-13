---
title: SegmentTreeLazy
description: "Lazy segment tree data structure from tyrus:algorithm."
---

# SegmentTreeLazy

`SegmentTreeLazy` implements a segment tree over a fixed-size array of 32-bit integers, supporting range queries and range additions in O(log n) through lazy propagation.

## Construction

| Syntax | Description |
|--------|-------------|
| `SegmentTreeLazy(data)` | Create a tree over the values in `data` |

The tree is built from a non-generic array of `i32` values. Positions are zero-indexed from `0` to `len - 1`. The length may be zero.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `updateRange` | `(from: usize, to: usize, delta: i32) => void` | `void` | Add `delta` to every element in `[from, to)` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Return the sum of the range `[from, to)` |
| `len` | `() => usize` | `usize` | Return the number of positions |
| `isEmpty` | `() => boolean` | `boolean` | Whether the tree has no positions |

Ranges use an exclusive upper bound: `query(from, to)` sums the elements at positions `from` through `to - 1`, and `updateRange(from, to, delta)` adds `delta` to those same positions. `query(k, k)` returns `0` and `updateRange(k, k, delta)` has no effect. Both range methods abort with a runtime error when `from > to` or `to > len`.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `updateRange` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Examples

### Construction and query

<!-- @include: ../../.fragments/modules/algorithm/segment-tree-lazy/construction-and-query.md -->

### updateRange

<!-- @include: ../../.fragments/modules/algorithm/segment-tree-lazy/updaterange.md -->

### Negative deltas and empty trees

<!-- @include: ../../.fragments/modules/algorithm/segment-tree-lazy/negative-deltas-and-empty-trees.md -->
