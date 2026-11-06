---
title: Hierholzer Eulerian
description: "Travessia Euleriana de Hierholzer em tyrus:algorithm."
---

# Hierholzer Eulerian

`hierholzerEulerian(graph)` constrói um caminho que usa cada aresta exatamente uma vez, unindo ciclos fechados. Aceita uma lista não direcionada ou `Graph<N, W>` e retorna ausência quando o caminho não existe.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `hierholzerEulerian(graph: usize[][])` | `usize[]?` | O(V + E) |
| `hierholzerEulerian(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

A lista deve representar um grafo não direcionado com entradas simétricas. Um caminho bem-sucedido tem `E + 1` vértices.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/hierholzer-eulerian/example.md -->

## Resultado para estudo

`hierholzerEulerianInfo` retorna o caminho junto com métricas de arestas e operações da pilha. Importe explicitamente as interfaces base e especializada.

<!-- @include: ../../../.fragments/modules/algorithm/hierholzer-eulerian/study-result.md -->
