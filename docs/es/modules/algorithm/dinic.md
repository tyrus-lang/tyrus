---
title: dinic
description: "Flujo máximo de Dinic en tyrus:algorithm."
---

# dinic

`dinic(capacity, source, sink)` calcula el flujo máximo usando grafos de niveles y flujos bloqueantes.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `dinic(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/dinic/dinic.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/dinic/study-result.md -->
