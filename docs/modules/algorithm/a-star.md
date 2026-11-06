---
title: A*
description: "Heuristic shortest-path search from tyrus:algorithm."
---

# A*

`aStar(graph, start, end, heuristic)` searches for a shortest path using one `i32` heuristic value per vertex. An admissible heuristic preserves optimality; the input may be an adjacency list or `Graph<N, i32>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `aStar(adj: [usize, i32][][], start: usize, end: usize, heuristic: i32[])` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../.fragments/modules/algorithm/a-star/a.md -->

## Study result

`aStarInfo` exposes distances, predecessors, relaxations, heuristic evaluations, opened and closed vertices, and the final cost when the destination is reached.

<!-- @include: ../../.fragments/modules/algorithm/a-star/study-result.md -->
