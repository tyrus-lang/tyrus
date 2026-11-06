---
title: topologicalSortDfs
description: "DFS topological sorting from tyrus:algorithm."
---

# topologicalSortDfs

`topologicalSortDfs(graph)` computes a topological ordering with depth-first traversal. It accepts an adjacency list or `Graph<N, W>` and returns absence if a back edge reveals a cycle.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `topologicalSortDfs(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortDfs(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

Every vertex appears once in the ordering. A graph with a directed cycle has no topological ordering and produces an empty optional result.

## Example

<!-- @include: ../../.fragments/modules/algorithm/topological-sort-dfs/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/topological-sort-dfs/study-result.md -->
