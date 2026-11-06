---
title: biconnectedComponents
description: "Descomposición en componentes biconexos en tyrus:algorithm."
---

# biconnectedComponents

`biconnectedComponents(graph)` descompone un grafo no dirigido en componentes biconexos de aristas mediante un recorrido de bajo enlace. Acepta una lista de adyacencia o `Graph<N, W>`.

| Firma | Retorno | Complejidad |
|-----------|---------|------------|
| `biconnectedComponents(graph: usize[][])` | `[usize, usize][][]` | O(V + E) |
| `biconnectedComponents(graph: Graph<N, W>)` | `[usize, usize][][]` | O(V + E) |

Cada array interno contiene las aristas de un componente como pares de vértices. Los puentes forman componentes unitarios.

## Ejemplo

<!-- @include: ../../../.fragments/modules/algorithm/biconnected-components/example.md -->

## Resultado para estudio

`biconnectedComponentsInfo` devuelve `BiconnectedComponentsInfo`, con componentes copiados y métricas low-link compartidas.

<!-- @include: ../../../.fragments/modules/algorithm/biconnected-components/study-result.md -->
