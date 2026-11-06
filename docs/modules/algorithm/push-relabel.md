---
title: pushRelabel
description: "Push-relabel maximum flow from tyrus:algorithm."
---

# pushRelabel

`pushRelabel(capacity, source, sink)` computes maximum flow using vertex heights and excess flow.

| Signature | Returns | Complexity |
|-----------|---------|------------|
| `pushRelabel(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../.fragments/modules/algorithm/push-relabel/pushrelabel.md -->

## Study result

<!-- @include: ../../.fragments/modules/algorithm/push-relabel/study-result.md -->
