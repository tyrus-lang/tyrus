---
title: A*
description: "Búsqueda heurística de ruta más corta de tyrus:algorithm."
---

# A*

`aStar(graph, start, end, heuristic)` busca una ruta más corta usando un valor heurístico `i32` por vértice. Una heurística admisible conserva la optimalidad.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `aStar(adj: [usize, i32][][], start: usize, end: usize, heuristic: i32[])` | `[usize[], i32]?` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/a-star/a.md -->

## Resultado de estudio

`aStarInfo` expone distancias, predecesores, relajaciones, evaluaciones de la heurística, vértices abiertos y cerrados y el costo final cuando se alcanza el destino.

<!-- @include: ../../../.fragments/modules/algorithm/a-star/study-result.md -->
