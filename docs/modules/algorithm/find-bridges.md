---
title: findBridges
description: "Bridge detection from tyrus:algorithm."
---

# findBridges

`findBridges(graph)` returns every undirected edge whose removal increases the number of connected components. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `findBridges(graph: usize[][])` | `[usize, usize][]` | O(V + E) |
| `findBridges(graph: Graph<N, W>)` | `[usize, usize][]` | O(V + E) |

The adjacency list is interpreted as undirected and should contain symmetric entries. Each result is a pair of vertex indices.

## Example

<!-- @include: ../../.fragments/modules/algorithm/find-bridges/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/connectivity-info.md -->
