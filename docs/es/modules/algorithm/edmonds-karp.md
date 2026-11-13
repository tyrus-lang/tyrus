---
title: edmondsKarp
description: "Flujo máximo de Edmonds-Karp en tyrus:algorithm."
---

# edmondsKarp

`edmondsKarp(capacity, source, sink)` calcula el flujo máximo en una matriz de capacidades y devuelve el valor y las aristas de flujo positivas.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `edmondsKarp(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(VE²) |

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/edmondskarp.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/study-result.md -->

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/study-result-2.md -->
