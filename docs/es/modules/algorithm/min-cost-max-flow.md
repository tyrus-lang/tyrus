---
title: minCostMaxFlow
description: "Flujo máximo de coste mínimo en tyrus:algorithm."
---

# minCostMaxFlow

`minCostMaxFlow(edges, vertexCount, source, sink)` aumenta caminos residuales con costes y devuelve el flujo y el coste totales.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `minCostMaxFlow(edges: [usize, usize, i32, i32][], vertexCount: usize, source: usize, sink: usize)` | `[i32, i32]` | O(FVE) |

<!-- @include: ../../../.fragments/modules/algorithm/min-cost-max-flow/mincostmaxflow.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/min-cost-max-flow/study-result.md -->
