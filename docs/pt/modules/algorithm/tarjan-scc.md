---
title: tarjanScc
description: "Componentes fortemente conexos de Tarjan em tyrus:algorithm."
---

# tarjanScc

`tarjanScc(graph)` divide um grafo direcionado em componentes fortemente conexos usando a travessia de baixo elo de Tarjan. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `tarjanScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `tarjanScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Cada array interno contém vértices que alcançam uns aos outros. A ordem depende da travessia.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/tarjan-scc/example.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/tarjan-scc/study-result.md -->
