---
title: FenwickTree
description: "Fenwick tree data structure from tyrus:algorithm."
---

# FenwickTree

`FenwickTree` implements a Fenwick tree (binary indexed tree) over a fixed-size array of 32-bit integers, supporting prefix sums and point updates in O(log n).

## Construction

| Syntax | Description |
|--------|-------------|
| `FenwickTree(n)` | Create a tree over `n` zero-initialized positions |

Positions are zero-indexed from `0` to `n - 1`. The length `n` may be zero.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `add` | `(index: usize, delta: i32) => void` | `void` | Add `delta` to the element at `index` |
| `sum` | `(index: usize) => i32` | `i32` | Return the sum of the first `index` elements, that is the range `[0, index)` |
| `len` | `() => usize` | `usize` | Return the number of positions |
| `isEmpty` | `() => boolean` | `boolean` | Whether the tree has no positions |

`sum(index)` returns the sum of the first `index` elements (exclusive upper bound): `sum(0)` is `0` and `sum(n)` is the total over all positions. `add` aborts with a runtime error when `index >= n`; `sum` aborts when `index > n`.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `add` | O(log n) |
| `sum` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/fenwick-tree/construction.md -->

### add

<!-- @include: ../../.fragments/modules/algorithm/fenwick-tree/add.md -->

### sum

<!-- @include: ../../.fragments/modules/algorithm/fenwick-tree/sum.md -->

### Negative deltas

<!-- @include: ../../.fragments/modules/algorithm/fenwick-tree/negative-deltas.md -->

### len and isEmpty

<!-- @include: ../../.fragments/modules/algorithm/fenwick-tree/len-and-isempty.md -->
