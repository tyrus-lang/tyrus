---
title: topologicalSortKahn
description: "Ordenación topológica de Kahn en tyrus:algorithm."
---

# topologicalSortKahn

`topologicalSortKahn(graph)` devuelve un orden topológico de un grafo acíclico dirigido. Acepta una lista de adyacencia o `Graph<N, W>` y devuelve ausencia cuando un ciclo impide completar el orden.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `topologicalSortKahn(graph: usize[][])` | `usize[]?` | O(V + E) |
| `topologicalSortKahn(graph: Graph<N, W>)` | `usize[]?` | O(V + E) |

El resultado contiene cada vértice una vez. Un resultado opcional vacío indica un ciclo dirigido.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-kahn/example.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/topological-sort-kahn/study-result.md -->
