---
title: prim
description: "Prim minimum spanning forest from tyrus:algorithm."
---

# prim

`prim(adjacency, origin)` grows a minimum spanning tree from `origin` with a priority queue. For disconnected inputs it returns the component reachable from `origin`.

Both `adjacency` and `primInfo` also accept `Graph<N, i32>`; add both directions of each undirected edge to a `Graph`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `prim(adjacency: [usize, i32][][], origin: usize)` | `[usize, usize, i32][]` | O((V + E) log V) |

<!-- @include: ../../.fragments/modules/algorithm/prim/prim.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/prim/study-result.md -->
