---
title: Bellman-Ford
description: "Bellman-Ford shortest paths from tyrus:algorithm."
---

# Bellman-Ford

`bellmanFord(graph, start)` returns distances and predecessor vertices and supports negative edge weights. A negative cycle returns `Err(AlgorithmError)` with code `ERR_ALGORITHM_NEGATIVE_CYCLE`; propagate it with `?` or handle it with `match`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `bellmanFord(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | O(VE) time, O(V) memory |

<!-- @include: ../../.fragments/modules/algorithm/bellman-ford/bellman-ford.md -->

## Study result

`bellmanFordInfo` returns every pass and relaxation metric and reports a reachable negative cycle as data, including the first edge that still permits relaxation.

<!-- @include: ../../.fragments/modules/algorithm/bellman-ford/study-result.md -->
