---
title: Johnson
description: "Rutas más cortas entre todos los pares en grafos dispersos de tyrus:algorithm."
---

# Johnson

`johnson(graph)` calcula rutas más cortas entre todos los pares en grafos ponderados dispersos combinando reponderación con Dijkstra. Un ciclo negativo devuelve `Err(AlgorithmError)` con el código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propágalo con `?` o manéjalo con `match`.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `johnson(adj: [usize, i32][][])` | `Result<(i32?)[][], AlgorithmError>` | O(VE + V(V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/johnson/johnson.md -->

## Resultado de estudio

`johnsonInfo` devuelve `Result<JohnsonInfo, AlgorithmError>` con la matriz de distancias de todos los pares, el conteo de reponderaciones, las relajaciones del Bellman-Ford inicial y las ejecuciones de Dijkstra.

<!-- @include: ../../../.fragments/modules/algorithm/johnson/study-result.md -->
