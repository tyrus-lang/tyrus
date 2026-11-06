---
title: findBridges
description: "Detecção de pontes em tyrus:algorithm."
---

# findBridges

`findBridges(graph)` retorna cada aresta não direcionada cuja remoção aumenta o número de componentes conexos. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `findBridges(graph: usize[][])` | `[usize, usize][]` | O(V + E) |
| `findBridges(graph: Graph<N, W>)` | `[usize, usize][]` | O(V + E) |

A lista deve conter entradas simétricas. Cada resultado é um par de índices de vértices.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/find-bridges/example.md -->

## Resultado para estudo

<!-- @include: ../../../.fragments/modules/algorithm/connectivity-info.md -->
