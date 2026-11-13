---
title: bfs
description: "Breadth-first graph traversal from tyrus:algorithm."
---

# bfs

`bfs(graph, start)` visits the vertices reachable from `start` in breadth-first order. The input can be an adjacency list or a `Graph<N, i32>`; the result contains vertex indices in visitation order.

## Function

| Signature | Returns | Description |
|-----------|---------|-------------|
| `bfs(adj: usize[][], start: usize)` | `usize[]` | Visits an adjacency-list graph in breadth-first order |
| `bfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visits a `Graph` in breadth-first order |

The starting vertex must be within the input graph. The traversal ignores vertices that are not reachable from `start` and marks each reachable vertex at most once.

## Complexity

| Operation | Time Complexity |
|-----------|-----------------|
| traversal | O(V + E) |
| memory | O(V) |

## Examples

### adjacency list

<!-- @include: ../../.fragments/modules/algorithm/bfs/adjacency-list.md -->

## Study result

`bfsInfo` returns `TraversalInfo` with the visit order and deterministic frontier, depth, and edge metrics.

<!-- @include: ../../.fragments/modules/algorithm/bfs/study-result.md -->

### Graph input

<!-- @include: ../../.fragments/modules/algorithm/bfs/graph-input.md -->
