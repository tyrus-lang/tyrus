---
title: dinic
description: "Dinic maximum flow from tyrus:algorithm."
---

# dinic

`dinic(capacity, source, sink)` computes maximum flow with level graphs and blocking flows.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `dinic(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../.fragments/modules/algorithm/dinic/dinic.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/dinic/study-result.md -->
