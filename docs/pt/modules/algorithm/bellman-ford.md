---
title: Bellman-Ford
description: "Menores caminhos de Bellman-Ford de tyrus:algorithm."
---

# Bellman-Ford

`bellmanFord(graph, start)` retorna distâncias e predecessores e aceita pesos negativos. Um ciclo negativo retorna `Err(AlgorithmError)` com o código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propague-o com `?` ou trate-o com `match`.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `bellmanFord(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | tempo O(VE), memória O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/bellman-ford/bellman-ford.md -->

## Resultado de estudo

`bellmanFordInfo` retorna as métricas de todas as passagens e relaxações e relata um ciclo negativo alcançável como dado, incluindo a primeira aresta que ainda permite relaxação.

<!-- @include: ../../../.fragments/modules/algorithm/bellman-ford/study-result.md -->
