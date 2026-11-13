---
title: edmondsBlossom
description: "General graph maximum matching from tyrus:algorithm."
---

# edmondsBlossom

`edmondsBlossom(graph)` computes a maximum matching in a general undirected graph, including graphs with odd cycles. It accepts an adjacency list or `Graph<N, W>`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `edmondsBlossom(graph: usize[][])` | `[usize, usize][]` | O(V³) |
| `edmondsBlossom(graph: Graph<N, W>)` | `[usize, usize][]` | O(V³) |

The result contains disjoint matched vertex pairs. The order of pairs and the choice among equivalent maximum matchings are not guaranteed.

## Example

<!-- @include: ../../.fragments/modules/algorithm/edmonds-blossom/example.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/edmonds-blossom/study-result.md -->
