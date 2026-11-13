---
title: Floyd-Warshall
description: "Dense all-pairs shortest paths from tyrus:algorithm."
---

# Floyd-Warshall

`floydWarshall(graph)` computes shortest distances between every pair. The input may be an `(i32?)[][]` weight matrix or `Graph<N, i32>`; absent entries represent missing edges.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `floydWarshall(matrix: (i32?)[][])` | `(i32?)[][]` | O(V³) time, O(V²) memory |

<!-- @include: ../../.fragments/modules/algorithm/floyd-warshall/floyd-warshall.md -->

## Study result

`floydWarshallInfo` returns the final matrix, the number of intermediate-vertex passes and distance updates, and whether the final diagonal reveals a negative cycle.

<!-- @include: ../../.fragments/modules/algorithm/floyd-warshall/study-result.md -->
