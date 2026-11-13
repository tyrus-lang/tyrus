---
title: edmondsKarp
description: "Edmonds-Karp maximum flow from tyrus:algorithm."
---

# edmondsKarp

`edmondsKarp(capacity, source, sink)` computes maximum flow in a capacity matrix and returns the value plus positive flow edges.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `edmondsKarp(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(VE²) |

<!-- @include: ../../.fragments/modules/algorithm/edmonds-karp/edmondskarp.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/edmonds-karp/study-result.md -->

<!-- @include: ../../.fragments/modules/algorithm/edmonds-karp/study-result-2.md -->
