---
title: Fleury Eulerian
description: "Recorrido Euleriano de Fleury en tyrus:algorithm."
---

# Fleury Eulerian

`fleuryEulerian(graph)` devuelve un camino que usa cada arista exactamente una vez, eligiendo aristas que no son puentes mientras haya alternativas. Acepta una lista no dirigida o `Graph<N, W>` y devuelve ausencia si no existe.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `fleuryEulerian(graph: usize[][])` | `usize[]?` | O(E(V + E)) |
| `fleuryEulerian(graph: Graph<N, W>)` | `usize[]?` | O(E(V + E)) |

La lista debe representar un grafo no dirigido con entradas simétricas. Un camino válido tiene `E + 1` vértices.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/fleury-eulerian/example.md -->

## Resultado para estudio

`fleuryEulerianInfo` devuelve el camino junto con métricas de examen de aristas y elecciones de puentes. Importe explícitamente las interfaces base y especializada.

<!-- @include: ../../../.fragments/modules/algorithm/fleury-eulerian/study-result.md -->
