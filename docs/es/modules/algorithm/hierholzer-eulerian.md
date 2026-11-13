---
title: Hierholzer Eulerian
description: "Recorrido Euleriano de Hierholzer en tyrus:algorithm."
---

# Hierholzer Eulerian

`hierholzerEulerian(graph)` construye un camino que usa cada arista exactamente una vez uniendo ciclos cerrados. Acepta una lista no dirigida o `Graph<N, W>` y devuelve ausencia si no existe.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `hierholzerEulerian(graph: usize[][])` | `usize[]?` | O(V + E) |
| `hierholzerEulerian(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

La lista debe representar un grafo no dirigido con entradas simétricas. Un camino válido tiene `E + 1` vértices.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/hierholzer-eulerian/example.md -->

## Resultado para estudio

`hierholzerEulerianInfo` devuelve el camino junto con métricas de aristas y operaciones de pila. Importe explícitamente las interfaces base y especializada.

<!-- @include: ../../../.fragments/modules/algorithm/hierholzer-eulerian/study-result.md -->
