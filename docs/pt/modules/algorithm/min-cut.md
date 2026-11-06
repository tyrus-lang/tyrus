---
title: minCut
description: "Corte mínimo em tyrus:algorithm."
---

# minCut

`minCut(capacity, source, sink)` calcula um fluxo máximo e retorna seu valor e as arestas de capacidade que cruzam o corte mínimo correspondente.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `minCut(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/min-cut/mincut.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/min-cut/study-result.md -->
