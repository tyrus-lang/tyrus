---
title: edmondsBlossom
description: "Emparejamiento máximo en grafos generales en tyrus:algorithm."
---

# edmondsBlossom

`edmondsBlossom(graph)` calcula un emparejamiento máximo en un grafo no dirigido general, incluidos los ciclos impares. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `edmondsBlossom(graph: usize[][])` | `[usize, usize][]` | O(V³) |
| `edmondsBlossom(graph: Graph<N, W>)` | `[usize, usize][]` | O(V³) |

El resultado contiene pares disjuntos de vértices emparejados. El orden de los pares no está garantizado.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-blossom/example.md -->

## Resultado para estudio

<!-- @include: ../../../.fragments/modules/algorithm/edmonds-blossom/study-result.md -->
