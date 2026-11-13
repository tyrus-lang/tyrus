---
title: tarjanScc
description: "Componentes fuertemente conexos de Tarjan en tyrus:algorithm."
---

# tarjanScc

`tarjanScc(graph)` divide un grafo dirigido en componentes fuertemente conexos mediante el recorrido de bajo enlace de Tarjan. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `tarjanScc(graph: usize[][])` | `usize[][]` | O(V + E) |
| `tarjanScc(graph: Graph<N, W>)` | `usize[][]` | O(V + E) |

Cada array interno contiene vértices que se alcanzan mutuamente. El orden depende del recorrido.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/tarjan-scc/example.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/tarjan-scc/study-result.md -->
