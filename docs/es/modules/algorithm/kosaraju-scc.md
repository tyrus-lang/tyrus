---
title: kosarajuScc
description: "Componentes fuertemente conexos de Kosaraju en tyrus:algorithm."
---

# kosarajuScc

`kosarajuScc(graph)` divide un grafo dirigido en componentes fuertemente conexos usando el orden de finalización y el grafo invertido. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `kosarajuScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `kosarajuScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Cada array interno es un conjunto maximal de vértices mutuamente alcanzables.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/kosaraju-scc/example.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/kosaraju-scc/study-result.md -->
