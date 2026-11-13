---
title: SPFA
description: "Queue-based shortest paths from tyrus:algorithm."
---

# SPFA

`spfa(graph, start)` computes single-source shortest paths with negative weights and returns distances plus predecessors. A negative cycle returns `Err(AlgorithmError)` with code `ERR_ALGORITHM_NEGATIVE_CYCLE`; propagate it with `?` or handle it with `match`.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `spfa(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | O(VE) worst case, O(V) memory |

<!-- @include: ../../.fragments/modules/algorithm/spfa/spfa.md -->

## Study result

`spfaInfo` returns `Result<SpfaInfo, AlgorithmError>` and adds queue pushes, queue pops, maximum queue size, and the shared shortest-path metrics. A reachable negative cycle is returned as `Err(AlgorithmError)`.

<!-- @include: ../../.fragments/modules/algorithm/spfa/study-result.md -->
