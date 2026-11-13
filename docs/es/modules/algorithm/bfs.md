---
title: bfs
description: "Recorrido de grafos en anchura de tyrus:algorithm."
---

# bfs

`bfs(graph, start)` visita los vértices alcanzables desde `start` en orden de anchura. La entrada puede ser una lista de adyacencia o un `Graph<N, i32>`; el resultado contiene los índices de los vértices en orden de visita.

## Función

| Firma | Devuelve | Descripción |
|-------|----------|-------------|
| `bfs(adj: usize[][], start: usize)` | `usize[]` | Visita un grafo de lista de adyacencia en orden de anchura |
| `bfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visita un `Graph` en orden de anchura |

El vértice inicial debe estar dentro del grafo de entrada. El recorrido ignora los vértices que no son alcanzables desde `start` y marca cada vértice alcanzable como máximo una vez.

## Complejidad

| Operación | Complejidad temporal |
|-----------|----------------------|
| recorrido | O(V + E) |
| memoria | O(V) |

## Ejemplos

### lista de adyacencia

<!-- @include: ../../../.fragments/modules/algorithm/bfs/adjacency-list.md -->

## Resultado de estudio

`bfsInfo` devuelve `TraversalInfo` con el orden de visita y métricas deterministas de frontera, profundidad y aristas.

<!-- @include: ../../../.fragments/modules/algorithm/bfs/study-result.md -->

### entrada Graph

<!-- @include: ../../../.fragments/modules/algorithm/bfs/graph-input.md -->
