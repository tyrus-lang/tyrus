---
title: kruskal
description: "Árbol de expansión mínima de Kruskal en tyrus:algorithm."
---

# kruskal

`kruskal(edges, vertexCount)` ordena las aristas ponderadas y selecciona cada arista que une dos componentes distintos. Devuelve un bosque de expansión mínima cuando la entrada está desconectada.

Tanto `edges` como `kruskalInfo` también aceptan `Graph<N, i32>`; agregue ambas direcciones de cada arista no dirigida al `Graph`.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `kruskal(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log E) |

<!-- @include: ../../../.fragments/modules/algorithm/kruskal/kruskal.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/kruskal/study-result.md -->
