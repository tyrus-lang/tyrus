---
title: Fleury Eulerian
description: "Travessia Euleriana de Fleury em tyrus:algorithm."
---

# Fleury Eulerian

`fleuryEulerian(graph)` retorna um caminho que usa cada aresta exatamente uma vez, escolhendo arestas que não são pontes enquanto houver alternativas. Aceita uma lista não direcionada ou `Graph<N, W>` e retorna ausência quando o caminho não existe.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `fleuryEulerian(graph: usize[][])` | `usize[]?` | O(E(V + E)) |
| `fleuryEulerian(graph: Graph<N, W>)` | `usize[]?` | O(E(V + E)) |

A lista deve representar um grafo não direcionado com entradas simétricas. Um caminho bem-sucedido tem `E + 1` vértices.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/fleury-eulerian/example.md -->

## Resultado para estudo

`fleuryEulerianInfo` retorna o caminho junto com métricas de exame de arestas e escolhas de pontes. Importe explicitamente as interfaces base e especializada.

<!-- @include: ../../../.fragments/modules/algorithm/fleury-eulerian/study-result.md -->
