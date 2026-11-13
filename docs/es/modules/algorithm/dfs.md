---
title: dfs
description: "Recorrido iterativo de grafos en profundidad de tyrus:algorithm."
---

# dfs

`dfs(graph, start)` visita los vértices alcanzables desde `start` en orden iterativo de profundidad. La entrada puede ser una lista de adyacencia o un `Graph<N, i32>`; el resultado contiene los índices de los vértices en orden de visita.

## Función

| Firma | Devuelve | Descripción |
|-------|----------|-------------|
| `dfs(adj: usize[][], start: usize)` | `usize[]` | Visita un grafo de lista de adyacencia en orden iterativo de profundidad |
| `dfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visita un `Graph` en orden iterativo de profundidad |

El vértice inicial debe estar dentro del grafo de entrada. Los vecinos se procesan en orden inverso de inserción en la pila para que el resultado siga el mismo orden que un recorrido recursivo sobre la lista de adyacencia.

## Complejidad

| Operación | Complejidad temporal |
|-----------|----------------------|
| recorrido | O(V + E) |
| memoria | O(V) |

## Ejemplos

### lista de adyacencia

<!-- @include: ../../../.fragments/modules/algorithm/dfs/adjacency-list.md -->

## Resultado de estudio

`dfsInfo` devuelve `TraversalInfo` con el orden de visita y métricas deterministas de pila, profundidad y aristas.

<!-- @include: ../../../.fragments/modules/algorithm/dfs/study-result.md -->

### entrada Graph

<!-- @include: ../../../.fragments/modules/algorithm/dfs/graph-input.md -->
