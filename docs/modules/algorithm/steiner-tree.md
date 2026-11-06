---
title: steinerTree
description: "Weighted terminal connection tree from tyrus:algorithm."
---

# steinerTree

`steinerTree(edges, vertexCount, terminals)` connects the requested terminals through weighted undirected paths and returns the selected edges. An empty terminal list returns an empty array; unreachable terminals also return an empty array.

Both `edges` and `steinerTreeInfo` also accept `Graph<N, i32>`; add both directions of each undirected edge to a `Graph`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `steinerTree(edges: [usize, usize, i32][], vertexCount: usize, terminals: usize[])` | `[usize, usize, i32][]` | O(T · (E + V) log V + E log V) |

<!-- @include: ../../.fragments/modules/algorithm/steiner-tree/steinertree.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/steiner-tree/study-result.md -->
