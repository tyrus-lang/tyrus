---
title: boruvka
description: "Árvore geradora mínima de Boruvka em tyrus:algorithm."
---

# boruvka

`boruvka(edges, vertexCount)` escolhe repetidamente a aresta de saída mais barata de cada componente. Retorna uma floresta geradora mínima, com uma árvore para cada componente conexo.

Tanto `edges` quanto `boruvkaInfo` também aceitam `Graph<N, i32>`; adicione as duas direções de cada aresta não direcionada ao `Graph`.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `boruvka(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log V) |

<!-- @include: ../../../.fragments/modules/algorithm/boruvka/boruvka.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/boruvka/study-result.md -->
