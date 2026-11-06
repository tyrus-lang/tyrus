---
title: Johnson
description: "Sparse all-pairs shortest paths from tyrus:algorithm."
---

# Johnson

`johnson(graph)` computes all-pairs shortest paths for sparse weighted graphs by combining reweighting with Dijkstra. A negative cycle returns `Err(AlgorithmError)` with code `ERR_ALGORITHM_NEGATIVE_CYCLE`; propagate it with `?` or handle it with `match`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `johnson(adj: [usize, i32][][])` | `Result<(i32?)[][], AlgorithmError>` | O(VE + V(V + E) log V) |

<!-- @include: ../../.fragments/modules/algorithm/johnson/johnson.md -->

## Study result

`johnsonInfo` returns `Result<JohnsonInfo, AlgorithmError>` with the all-pairs distance matrix, reweighting count, initial Bellman-Ford relaxation count, and Dijkstra execution count.

<!-- @include: ../../.fragments/modules/algorithm/johnson/study-result.md -->
