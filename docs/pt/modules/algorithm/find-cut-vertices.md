---
title: findCutVertices
description: "Detecção de vértices de articulação em tyrus:algorithm."
---

# findCutVertices

`findCutVertices(graph)` retorna vértices cuja remoção desconecta um grafo não direcionado. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `findCutVertices(graph: usize[][])` | `usize[]` | O(V + E) |
| `findCutVertices(graph: Graph<N, W>)` | `usize[]` | O(V + E) |

A lista deve conter entradas simétricas. Cada vértice aparece uma única vez.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/find-cut-vertices/example.md -->

## Resultado para estudo

`findCutVerticesInfo` retorna `FindCutVerticesInfo`, com os vértices de corte copiados e métricas low-link compartilhadas.

<!-- @include: ../../../.fragments/modules/algorithm/find-cut-vertices/study-result.md -->
