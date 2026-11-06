---
title: dfs
description: "Travessia iterativa de grafos em profundidade de tyrus:algorithm."
---

# dfs

`dfs(graph, start)` visita os vértices alcançáveis a partir de `start` em ordem iterativa de profundidade. A entrada pode ser uma lista de adjacência ou um `Graph<N, i32>`; o resultado contém os índices dos vértices na ordem de visita.

## Função

| Assinatura | Retorna | Descrição |
|------------|---------|-----------|
| `dfs(adj: usize[][], start: usize)` | `usize[]` | Visita um grafo em lista de adjacência em ordem iterativa de profundidade |
| `dfs(graph: Graph<N, i32>, start: usize)` | `usize[]` | Visita um `Graph` em ordem iterativa de profundidade |

O vértice inicial deve estar dentro do grafo de entrada. Os vizinhos são processados na ordem inversa de inserção na pilha, para que o resultado siga a mesma ordem de uma travessia recursiva sobre a lista de adjacência.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| travessia | O(V + E) |
| memória | O(V) |

## Exemplos

### lista de adjacência

<!-- @include: ../../../.fragments/modules/algorithm/dfs/adjacency-list.md -->

## Resultado de estudo

`dfsInfo` retorna `TraversalInfo` com a ordem de visita e métricas determinísticas de pilha, profundidade e arestas.

<!-- @include: ../../../.fragments/modules/algorithm/dfs/study-result.md -->

### entrada Graph

<!-- @include: ../../../.fragments/modules/algorithm/dfs/graph-input.md -->
