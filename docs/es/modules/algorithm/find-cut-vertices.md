---
title: findCutVertices
description: "Detección de vértices de articulación en tyrus:algorithm."
---

# findCutVertices

`findCutVertices(graph)` devuelve los vértices cuya eliminación desconecta un grafo no dirigido. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `findCutVertices(graph: usize[][])` | `usize[]` | O(V + E) |
| `findCutVertices(graph: Graph<N, W>)` | `usize[]` | O(V + E) |

La lista debe contener entradas simétricas. Cada vértice aparece una sola vez.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/find-cut-vertices/example.md -->

## Resultado para estudio

`findCutVerticesInfo` devuelve `FindCutVerticesInfo`, con vértices de corte copiados y métricas low-link compartidas.

<!-- @include: ../../../.fragments/modules/algorithm/find-cut-vertices/study-result.md -->
