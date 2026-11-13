---
title: kruskal
description: "Árvore geradora mínima de Kruskal em tyrus:algorithm."
---

# kruskal

`kruskal(edges, vertexCount)` ordena as arestas ponderadas e seleciona cada aresta que une dois componentes distintos. Retorna uma floresta geradora mínima quando a entrada é desconexa.

Tanto `edges` quanto `kruskalInfo` também aceitam `Graph<N, i32>`; adicione as duas direções de cada aresta não direcionada ao `Graph`.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `kruskal(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log E) |

<!-- @include: ../../../.fragments/modules/algorithm/kruskal/kruskal.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/kruskal/study-result.md -->
