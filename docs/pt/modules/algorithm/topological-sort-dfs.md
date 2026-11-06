---
title: topologicalSortDfs
description: "Ordenação topológica por DFS em tyrus:algorithm."
---

# topologicalSortDfs

`topologicalSortDfs(graph)` calcula uma ordenação topológica com busca em profundidade. Aceita uma lista de adjacência ou `Graph<N, W>` e retorna ausência quando uma aresta de retorno revela um ciclo.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `topologicalSortDfs(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortDfs(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

Cada vértice aparece uma vez. Um grafo com ciclo direcionado não possui ordenação topológica.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-dfs/example.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-dfs/study-result.md -->
