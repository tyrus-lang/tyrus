---
title: kosarajuScc
description: "Componentes fortemente conexos de Kosaraju em tyrus:algorithm."
---

# kosarajuScc

`kosarajuScc(graph)` divide um grafo direcionado em componentes fortemente conexos usando a ordem de finalização e o grafo reverso. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `kosarajuScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `kosarajuScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Cada array interno é um conjunto maximal de vértices mutuamente alcançáveis.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/kosaraju-scc/example.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/kosaraju-scc/study-result.md -->
