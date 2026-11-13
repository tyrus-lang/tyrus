---
title: A*
description: "Busca heurística de menor caminho de tyrus:algorithm."
---

# A*

`aStar(graph, start, end, heuristic)` procura um menor caminho usando um valor heurístico `i32` por vértice. Uma heurística admissível preserva a optimalidade.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `aStar(adj: [usize, i32][][], start: usize, end: usize, heuristic: i32[])` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/a-star/a.md -->

## Resultado de estudo

`aStarInfo` expõe distâncias, predecessores, relaxações, avaliações da heurística, vértices abertos e fechados e o custo final quando o destino é alcançado.

<!-- @include: ../../../.fragments/modules/algorithm/a-star/study-result.md -->
