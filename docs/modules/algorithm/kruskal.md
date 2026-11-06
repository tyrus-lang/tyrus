---
title: kruskal
description: "Kruskal minimum spanning forest from tyrus:algorithm."
---

# kruskal

`kruskal(edges, vertexCount)` sorts weighted edges and selects each edge that joins two disjoint components. It returns an edge list and supports disconnected inputs by returning a minimum spanning forest.

Both `edges` and `kruskalInfo` also accept `Graph<N, i32>`; add both directions of each undirected edge to a `Graph`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `kruskal(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log E) |

<!-- @include: ../../.fragments/modules/algorithm/kruskal/kruskal.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/kruskal/study-result.md -->
