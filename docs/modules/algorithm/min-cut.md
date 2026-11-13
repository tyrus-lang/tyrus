---
title: minCut
description: "Minimum cut from tyrus:algorithm."
---

# minCut

`minCut(capacity, source, sink)` computes a maximum flow and returns its value plus the capacity edges crossing the corresponding minimum cut.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `minCut(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize][]]` | O(V²E) |

<!-- @include: ../../.fragments/modules/algorithm/min-cut/mincut.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/min-cut/study-result.md -->
