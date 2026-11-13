---
title: Dijkstra
description: "Dijkstra shortest-path search from tyrus:algorithm."
---

# Dijkstra

`dijkstra(graph, start, end)` returns `[path, distance]` or absence when the destination is unreachable. Edges must have non-negative `i32` weights; the input may be a weighted adjacency list or `Graph<N, i32>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `dijkstra(adj: [usize, i32][][], start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |
| `dijkstra(graph: Graph<N, i32>, start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../.fragments/modules/algorithm/dijkstra/dijkstra.md -->

## Study result

`dijkstraInfo` returns the tentative distances, predecessors, accepted relaxations, priority-queue extractions, and finalized-vertex metrics for the same input forms.

<!-- @include: ../../.fragments/modules/algorithm/dijkstra/study-result.md -->
