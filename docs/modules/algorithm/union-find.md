---
title: UnionFind
description: "Disjoint set (union-find) data structure from tyrus:algorithm."
---

# UnionFind

`UnionFind<T>` implements a disjoint set (union-find) data structure with path compression and union by rank. It maintains a collection of non-overlapping sets and supports efficient union and find operations.

## Construction

| Syntax | Description |
|--------|-------------|
| `UnionFind<u32>(n)` | Create a new disjoint set with `n` elements indexed from `0` to `n - 1` |
| `UnionFind<u8>(n)` | Same, with `u8` element type |
| `UnionFind<u16>(n)` | Same, with `u16` element type |
| `UnionFind<u64>(n)` | Same, with `u64` element type |

Only unsigned integer types are allowed as the element type.

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `find` | `(T) => T` | `T` | Root representative of the set containing `x` |
| `union` | `(T, T) => boolean` | `boolean` | Merge sets; returns `true` if a merge occurred |
| `connected` | `(T, T) => boolean` | `boolean` | Whether `x` and `y` belong to the same set |
| `setCount` | `() => T` | `T` | Number of disjoint sets |

Elements are zero-indexed integers from `0` to `n - 1`. The native handle is released automatically when the binding goes out of scope.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| `find` | Amortized O(α(n)), nearly constant |
| `union` | Amortized O(α(n)), nearly constant |
| `connected` | Amortized O(α(n)), nearly constant |
| `setCount` | O(1) |

Where α is the inverse Ackermann function, which is effectively constant for all practical inputs.

## Examples

### Construction

<!-- @include: ../../.fragments/modules/algorithm/union-find/construction.md -->

### find

<!-- @include: ../../.fragments/modules/algorithm/union-find/find.md -->

### union

<!-- @include: ../../.fragments/modules/algorithm/union-find/union.md -->

### connected

<!-- @include: ../../.fragments/modules/algorithm/union-find/connected.md -->

### setCount

<!-- @include: ../../.fragments/modules/algorithm/union-find/setcount.md -->

