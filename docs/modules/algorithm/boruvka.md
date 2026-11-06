---
title: boruvka
description: "Boruvka minimum spanning forest from tyrus:algorithm."
---

# boruvka

`boruvka(edges, vertexCount)` repeatedly chooses the cheapest outgoing edge of every component. It returns a minimum spanning forest, including one tree for each connected component.

Both `edges` and `boruvkaInfo` also accept `Graph<N, i32>`; add both directions of each undirected edge to a `Graph`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `boruvka(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log V) |

<!-- @include: ../../.fragments/modules/algorithm/boruvka/boruvka.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/boruvka/study-result.md -->
