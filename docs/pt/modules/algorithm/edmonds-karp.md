---
title: edmondsKarp
description: "Fluxo máximo de Edmonds-Karp em tyrus:algorithm."
---

# edmondsKarp

`edmondsKarp(capacity, source, sink)` calcula o fluxo máximo em uma matriz de capacidades e retorna o valor e as arestas de fluxo positivas.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `edmondsKarp(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(VE²) |

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/edmondskarp.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/study-result.md -->

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-karp/study-result-2.md -->
