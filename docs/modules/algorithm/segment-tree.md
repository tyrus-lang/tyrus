---
title: SegmentTree
description: "Segment tree data structure from tyrus:algorithm."
---

# SegmentTree

`SegmentTree` implements a segment tree over a fixed-size array of 32-bit integers, supporting range queries and point updates in O(log n).

## Construction

| Syntax | Description |
|--------|-------------|
| `SegmentTree(data)` | Create a tree over the values in `data` |

The tree is built from a non-generic array of `i32` values. Positions are zero-indexed from `0` to `len - 1`. The length may be zero.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `update` | `(index: usize, value: i32) => void` | `void` | Replace the element at `index` with `value` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Return the sum of the range `[from, to)` |
| `len` | `() => usize` | `usize` | Return the number of positions |
| `isEmpty` | `() => boolean` | `boolean` | Whether the tree has no positions |

Ranges use an exclusive upper bound: `query(from, to)` sums the elements at positions `from` through `to - 1`, so `query(0, len)` is the total over all positions and `query(k, k)` is `0`. `update` aborts with a runtime error when `index >= len`; `query` aborts when `from > to` or `to > len`.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `update` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/segment-tree/construction.md -->

### query

<!-- @include: ../../.fragments/modules/algorithm/segment-tree/query.md -->

### update

<!-- @include: ../../.fragments/modules/algorithm/segment-tree/update.md -->

### Negative values

<!-- @include: ../../.fragments/modules/algorithm/segment-tree/negative-values.md -->

### len and isEmpty

<!-- @include: ../../.fragments/modules/algorithm/segment-tree/len-and-isempty.md -->
