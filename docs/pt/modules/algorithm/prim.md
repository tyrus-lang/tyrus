---
title: prim
description: "Árvore geradora mínima de Prim em tyrus:algorithm."
---

# prim

`prim(adjacency, origin)` constrói uma árvore geradora mínima a partir de `origin` usando uma fila de prioridade. Em entradas desconexas, retorna o componente alcançável a partir de `origin`.

Tanto `adjacency` quanto `primInfo` também aceitam `Graph<N, i32>`; adicione as duas direções de cada aresta não direcionada ao `Graph`.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `prim(adjacency: [usize, i32][][], origin: usize)` | `[usize, usize, i32][]` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/prim/prim.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/prim/study-result.md -->
