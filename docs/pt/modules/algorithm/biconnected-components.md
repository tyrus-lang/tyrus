---
title: biconnectedComponents
description: "Decomposição em componentes biconexos em tyrus:algorithm."
---

# biconnectedComponents

`biconnectedComponents(graph)` decompõe um grafo não direcionado em componentes biconexos de arestas usando uma travessia de baixo elo. Aceita uma lista de adjacência ou `Graph<N, W>`.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `biconnectedComponents(graph: usize[][])` | `[usize, usize][][]` | O(V + E) |
| `biconnectedComponents(graph: Graph<N, W>)` | `[usize, usize][][]` | O(V + E) |

Cada array interno contém as arestas de um componente como pares de vértices. Pontes formam componentes unitários.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/biconnected-components/example.md -->

## Resultado para estudo

`biconnectedComponentsInfo` retorna `BiconnectedComponentsInfo`, com componentes copiados e métricas low-link compartilhadas.

<!-- @include: ../../../.fragments/modules/algorithm/biconnected-components/study-result.md -->
