---
title: findBridges
description: "Detección de puentes en tyrus:algorithm."
---

# findBridges

`findBridges(graph)` devuelve cada arista no dirigida cuya eliminación aumenta el número de componentes conexos. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `findBridges(graph: usize[][])` | `[usize, usize][]` | O(V + E) |
| `findBridges(graph: Graph<N, W>)` | `[usize, usize][]` | O(V + E) |

La lista debe contener entradas simétricas. Cada resultado es un par de índices de vértices.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/find-bridges/example.md -->

## Resultado para estudio

<!-- @include: ../../../.fragments/modules/algorithm/connectivity-info.md -->
