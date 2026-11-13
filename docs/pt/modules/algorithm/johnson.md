---
title: Johnson
description: "Menores caminhos entre todos os pares em grafos esparsos de tyrus:algorithm."
---

# Johnson

`johnson(graph)` calcula menores caminhos entre todos os pares em grafos ponderados esparsos combinando reponderação com Dijkstra. Um ciclo negativo retorna `Err(AlgorithmError)` com o código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propague-o com `?` ou trate-o com `match`.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `johnson(adj: [usize, i32][][])` | `Result<(i32?)[][], AlgorithmError>` | O(VE + V(V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/johnson/johnson.md -->

## Resultado de estudo

`johnsonInfo` retorna `Result<JohnsonInfo, AlgorithmError>` com a matriz de distâncias de todos os pares, a contagem de reponderações, as relaxações do Bellman-Ford inicial e as execuções de Dijkstra.

<!-- @include: ../../../.fragments/modules/algorithm/johnson/study-result.md -->
