---
title: dinic
description: "Fluxo máximo de Dinic em tyrus:algorithm."
---

# dinic

`dinic(capacity, source, sink)` calcula o fluxo máximo usando grafos de níveis e fluxos bloqueadores.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `dinic(capacity: i32[][], source: usize, sink: usize)` | `[i32, [usize, usize, i32][]]` | O(V²E) |

<!-- @include: ../../../.fragments/modules/algorithm/dinic/dinic.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/dinic/study-result.md -->
