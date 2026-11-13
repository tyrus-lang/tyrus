---
title: tarjanScc
description: "Tarjan strongly connected components from tyrus:algorithm."
---

# tarjanScc

`tarjanScc(graph)` partitions a directed graph into strongly connected components with Tarjan's low-link traversal. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `tarjanScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `tarjanScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Each returned inner array contains vertex indices that can reach one another. Components are returned in traversal-dependent order.

## Example

<!-- @include: ../../.fragments/modules/algorithm/tarjan-scc/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/tarjan-scc/study-result.md -->
