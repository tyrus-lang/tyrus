---
title: centroidDecomposition
description: "Centroid decomposition of trees from tyrus:algorithm."
---

# centroidDecomposition

`centroidDecomposition(adj: usize[][]): Centroid` builds the centroid decomposition over the undirected tree described by the adjacency list `adj`, where `adj[v]` holds the neighbors of vertex `v`. The returned `Centroid` is an owned handle of `tyrus:algorithm`; every vertex of the input tree becomes a node of the centroid tree, and removing a centroid splits each component into child components of depth at most half its size. It exposes `parentOf(v)`, which returns the centroid-tree parent of `v` as `usize?` (the global centroid root has none), and `depth(v)`, the depth of `v` in the centroid tree (root depth 0).

## Construction

| Syntax | Description |
|--------|-------------|
| `centroidDecomposition(adj: usize[][])` | Decompose the tree `adj` into a centroid tree |

## Methods

| Method | Signature | Returns | Description |
|--------|-----------|---------|-------------|
| `parentOf` | `(v: usize)` | `usize?` | Centroid-tree parent of `v`; absent for the global centroid root |
| `depth` | `(v: usize)` | `usize` | Depth of `v` in the centroid tree (root depth 0) |

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| construction | O(n), iterative per-component subtree sizing and centroid search |
| `parentOf`, `depth` | O(1) |
| memory | O(n) |

## Examples

### balanced tree

<!-- @include: ../../.fragments/modules/algorithm/centroid/balanced-tree.md -->

### deep path

<!-- @include: ../../.fragments/modules/algorithm/centroid/deep-path.md -->

### single vertex

<!-- @include: ../../.fragments/modules/algorithm/centroid/single-vertex.md -->

### namespace import

<!-- @include: ../../.fragments/modules/algorithm/centroid/namespace-import.md -->
