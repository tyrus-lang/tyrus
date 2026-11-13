---
title: topologicalSortKahn
description: "Kahn topological sorting from tyrus:algorithm."
---

# topologicalSortKahn

`topologicalSortKahn(graph)` returns a topological ordering for a directed acyclic graph. It accepts an adjacency list or `Graph<N, W>` and returns absence when a cycle prevents a complete ordering.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `topologicalSortKahn(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortKahn(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

The result contains each vertex once. An empty optional result means that the input contains a directed cycle.

## Example

<!-- @include: ../../.fragments/modules/algorithm/topological-sort-kahn/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/topological-sort-kahn/study-result.md -->
