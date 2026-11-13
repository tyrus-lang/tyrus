---
title: SparseTable
description: "Immutable RMQ table from tyrus:algorithm."
---

# SparseTable

`SparseTable` precomputes an immutable range-minimum table over a fixed-size array of 32-bit integers, answering minimum queries in O(1) time after O(n log n) preprocessing.

## Construction

| Syntax | Description |
|--------|-------------|
| `SparseTable(data)` | Build an RMQ table over the values in `data` |

The table is built from a non-generic array of `i32` values. Positions are zero-indexed from `0` to `len - 1`. The length may be zero. Once constructed the table is immutable: use `SegmentTree` when you need point updates.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `query` | `(from: usize, to: usize) => i32` | `i32` | Return the minimum of the range `[from, to)` |
| `len` | `() => usize` | `usize` | Return the number of positions |
| `isEmpty` | `() => boolean` | `boolean` | Whether the table has no positions |

Ranges use an exclusive upper bound: `query(from, to)` returns the minimum of the elements at positions `from` through `to - 1`, so `query(0, len)` is the minimum over all positions. `query` aborts with a runtime error when `from >= to` or `to > len`.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `query` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/sparse-table/construction.md -->

### query

<!-- @include: ../../.fragments/modules/algorithm/sparse-table/query.md -->

### len and isEmpty

<!-- @include: ../../.fragments/modules/algorithm/sparse-table/len-and-isempty.md -->
