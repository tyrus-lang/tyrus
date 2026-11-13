---
title: heavyLightDecomposition
description: "Heavy-light decomposition of trees from tyrus:algorithm."
---

# heavyLightDecomposition

`heavyLightDecomposition(adj: usize[][], root: usize): Hld` builds a heavy-light decomposition over the undirected tree described by the adjacency list `adj`, where `adj[v]` holds the neighbors of vertex `v` and `root` anchors every query. The returned `Hld` is an owned handle of `tyrus:algorithm`; it exposes `queryPath(u, v)`, which returns the contiguous `[l, r]` ranges (in `position` order) whose union covers the tree path `u → v` in O(log n), ready to feed a segment tree for path queries.

## Construction

| Syntax | Description |
|--------|-------------|
| `heavyLightDecomposition(adj: usize[][], root: usize)` | Decompose the tree `adj` into heavy chains, rooted at `root` |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `queryPath` | `(u: usize, v: usize)` | `[usize, usize][]` | Chain ranges whose union covers the tree path `u → v` |
| `position` | `(v: usize)` | `usize` | Chain-linear position of vertex `v` |
| `head` | `(v: usize)` | `usize` | Topmost vertex of the heavy chain containing `v` |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| construction | O(n), one DFS plus chain assignment |
| `queryPath` | O(log n), at most O(log n) chain ranges |
| `position`, `head` | O(1) |
| memory | O(n) |

## Examples

### balanced tree

<!-- @include: ../../.fragments/modules/algorithm/hld/balanced-tree.md -->

### deep path

<!-- @include: ../../.fragments/modules/algorithm/hld/deep-path.md -->

### single vertex

<!-- @include: ../../.fragments/modules/algorithm/hld/single-vertex.md -->

### namespace import

<!-- @include: ../../.fragments/modules/algorithm/hld/namespace-import.md -->
