---
title: bfs
description: "Travessia de grafos em largura de tyrus:algorithm."
---

# bfs

`bfs(graph, start)` visita os vértices alcançáveis a partir de `start` em ordem de largura. A entrada pode ser uma lista de adjacência ou um `Graph<N, i32>`; o resultado contém os índices dos vértices na ordem de visita.

## Função

| Assinatura | Retorna | Descrição |
|------------|---------|-----------|
| `bfs(adj: usize[][], start: usize)` | `usize[]` | Visita um grafo em lista de adjacência em ordem de largura |
| `bfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visita um `Graph` em ordem de largura |

O vértice inicial deve estar dentro do grafo de entrada. A travessia ignora vértices que não são alcançáveis a partir de `start` e marca cada vértice alcançável no máximo uma vez.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| travessia | O(V + E) |
| memória | O(V) |

## Exemplos

### lista de adjacência

<!-- @include: ../../../.fragments/modules/algorithm/bfs/adjacency-list.md -->

## Resultado de estudo

`bfsInfo` retorna `TraversalInfo` com a ordem de visita e métricas determinísticas de fronteira, profundidade e arestas.

<!-- @include: ../../../.fragments/modules/algorithm/bfs/study-result.md -->

### entrada Graph

<!-- @include: ../../../.fragments/modules/algorithm/bfs/graph-input.md -->
