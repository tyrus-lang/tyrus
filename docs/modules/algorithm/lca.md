---
title: lcaBinaryLifting
description: "Lowest common ancestor tables for trees from tyrus:algorithm."
---

# lcaBinaryLifting

`lcaBinaryLifting(adj: usize[][], root: usize): Lca` builds binary lifting tables over the undirected tree described by the adjacency list `adj`, where `adj[v]` holds the neighbors of vertex `v` and `root` anchors every query. The returned `Lca` is an owned handle of `tyrus:algorithm`; it exposes `query(u, v)`, which returns the lowest common ancestor of vertices `u` and `v` in O(log n) with binary lifting.

## Construction

| Syntax | Description |
|--------|-------------|
| `lcaBinaryLifting(adj: usize[][], root: usize)` | Build LCA tables over the tree `adj`, rooted at `root` |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `query` | `(u: usize, v: usize)` | `usize` | Lowest common ancestor of vertices `u` and `v` |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| construction | O(n log n), binary lifting tables |
| `query` | O(log n) |
| memory | O(n log n) |

## Examples

### balanced tree

<!-- @include: ../../.fragments/modules/algorithm/lca/balanced-tree.md -->

### deep path

<!-- @include: ../../.fragments/modules/algorithm/lca/deep-path.md -->

### single vertex

<!-- @include: ../../.fragments/modules/algorithm/lca/single-vertex.md -->

### namespace import

<!-- @include: ../../.fragments/modules/algorithm/lca/namespace-import.md -->
