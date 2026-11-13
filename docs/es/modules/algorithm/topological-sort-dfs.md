---
title: topologicalSortDfs
description: "Ordenación topológica por DFS en tyrus:algorithm."
---

# topologicalSortDfs

`topologicalSortDfs(graph)` calcula un orden topológico mediante búsqueda en profundidad. Acepta una lista de adyacencia o `Graph<N, W>` y devuelve ausencia si una arista de retorno revela un ciclo.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `topologicalSortDfs(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortDfs(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

Cada vértice aparece una vez. Un grafo con ciclo dirigido no tiene orden topológico.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-dfs/example.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-dfs/study-result.md -->
