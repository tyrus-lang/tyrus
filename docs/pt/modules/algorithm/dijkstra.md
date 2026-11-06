---
title: Dijkstra
description: "Busca de menor caminho de Dijkstra de tyrus:algorithm."
---

# Dijkstra

`dijkstra(graph, start, end)` retorna `[path, distance]` ou ausência quando o destino é inalcançável. As arestas devem ter pesos `i32` não negativos; a entrada pode ser uma lista de adjacência ponderada ou `Graph<N, i32>`.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `dijkstra(adj: [usize, i32][][], start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |
| `dijkstra(graph: Graph<N, i32>, start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/dijkstra/dijkstra.md -->

## Resultado de estudo

`dijkstraInfo` retorna as distâncias provisórias, predecessores, relaxações aceitas, extrações da fila de prioridade e métricas de vértices finalizados para as mesmas formas de entrada.

<!-- @include: ../../../.fragments/modules/algorithm/dijkstra/study-result.md -->
