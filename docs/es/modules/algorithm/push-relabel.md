---
title: pushRelabel
description: "Flujo máximo push-relabel en tyrus:algorithm."
---

# pushRelabel

`pushRelabel(capacity, source, sink)` calcula el flujo máximo usando alturas de vértices y flujo excedente.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `pushRelabel(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/push-relabel/pushrelabel.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/push-relabel/study-result.md -->
