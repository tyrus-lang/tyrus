---
title: Floyd-Warshall
description: "Rutas más cortas entre todos los pares de tyrus:algorithm."
---

# Floyd-Warshall

`floydWarshall(graph)` calcula las distancias más cortas entre todos los pares. La entrada puede ser una matriz de pesos `(i32?)[][]` o `Graph<N, i32>`; las ausencias representan aristas inexistentes.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `floydWarshall(matrix: (i32?)[][])` | `(i32?)[][]` | tiempo O(V³), memoria O(V²) |

<!-- @include: ../../../.fragments/modules/algorithm/floyd-warshall/floyd-warshall.md -->

## Resultado de estudio

`floydWarshallInfo` devuelve la matriz final, el número de pasadas por vértice intermedio y actualizaciones de distancia y si la diagonal final revela un ciclo negativo.

<!-- @include: ../../../.fragments/modules/algorithm/floyd-warshall/study-result.md -->
