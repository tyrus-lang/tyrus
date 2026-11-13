---
title: topologicalSortKahn
description: "Ordenação topológica de Kahn em tyrus:algorithm."
---

# topologicalSortKahn

`topologicalSortKahn(graph)` retorna uma ordenação topológica de um grafo acíclico direcionado. Aceita uma lista de adjacência ou `Graph<N, W>` e retorna ausência quando um ciclo impede a ordenação completa.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `topologicalSortKahn(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortKahn(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

O resultado contém cada vértice uma vez. Um resultado opcional vazio indica um ciclo direcionado.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-kahn/example.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-kahn/study-result.md -->
