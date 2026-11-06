---
title: pushRelabel
description: "Fluxo máximo push-relabel em tyrus:algorithm."
---

# pushRelabel

`pushRelabel(capacity, source, sink)` calcula o fluxo máximo usando alturas de vértices e fluxo excedente.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `pushRelabel(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/push-relabel/pushrelabel.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/push-relabel/study-result.md -->
