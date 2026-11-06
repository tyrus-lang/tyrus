---
title: Dijkstra
description: "Búsqueda de ruta más corta de Dijkstra de tyrus:algorithm."
---

# Dijkstra

`dijkstra(graph, start, end)` devuelve `[path, distance]` o ausencia cuando el destino es inalcanzable. Las aristas deben tener pesos `i32` no negativos; la entrada puede ser una lista de adyacencia ponderada o `Graph<N, i32>`.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `dijkstra(adj: [usize, i32][][], start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |
| `dijkstra(graph: Graph<N, i32>, start: usize, end: usize)` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/dijkstra/dijkstra.md -->

## Resultado de estudio

`dijkstraInfo` devuelve las distancias provisionales, predecesores, relajaciones aceptadas, extracciones de la cola de prioridad y métricas de vértices finalizados para las mismas formas de entrada.

<!-- @include: ../../../.fragments/modules/algorithm/dijkstra/study-result.md -->
