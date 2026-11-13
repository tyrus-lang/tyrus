---
title: edmondsBlossom
description: "Emparelhamento máximo em grafos gerais em tyrus:algorithm."
---

# edmondsBlossom

`edmondsBlossom(graph)` calcula um emparelhamento máximo em um grafo não direcionado geral, inclusive com ciclos ímpares. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `edmondsBlossom(graph: usize[][])` | `[usize, usize][]` | O(V³) |
| `edmondsBlossom(graph: Graph<N, W>)` | `[usize, usize][]` | O(V³) |

O resultado contém pares disjuntos de vértices emparelhados. A ordem dos pares não é garantida.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-blossom/example.md -->

## Resultado para estudo

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-blossom/study-result.md -->
