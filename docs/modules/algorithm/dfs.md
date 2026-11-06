---
title: dfs
description: "Iterative depth-first graph traversal from tyrus:algorithm."
---

# dfs

`dfs(graph, start)` visits the vertices reachable from `start` in iterative depth-first order. The input can be an adjacency list or a `Graph<N, i32>`; the result contains vertex indices in visitation order.

## Function

| Signature | Returns | Description |
|-----------|---------|-------------|
| `dfs(adj: usize[][], start: usize)` | `usize[]` | Visits an adjacency-list graph in iterative depth-first order |
| `dfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visits a `Graph` in iterative depth-first order |

The starting vertex must be within the input graph. Neighbors are processed in reverse stack insertion order so the result follows the same order as a recursive traversal over the adjacency list.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| traversal | O(V + E) |
| memory | O(V) |

## Examples

### adjacency list

<!-- @include: ../../.fragments/modules/algorithm/dfs/adjacency-list.md -->

## Study result

`dfsInfo` returns `TraversalInfo` with the visit order and deterministic stack, depth, and edge metrics.

<!-- @include: ../../.fragments/modules/algorithm/dfs/study-result.md -->

### Graph input

<!-- @include: ../../.fragments/modules/algorithm/dfs/graph-input.md -->
