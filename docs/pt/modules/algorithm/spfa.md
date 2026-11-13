---
title: SPFA
description: "Menores caminhos baseados em fila de tyrus:algorithm."
---

# SPFA

`spfa(graph, start)` calcula menores caminhos a partir de uma origem com pesos negativos e retorna distâncias e predecessores. Um ciclo negativo retorna `Err(AlgorithmError)` com o código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propague-o com `?` ou trate-o com `match`.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `spfa(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | pior caso O(VE), memória O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/spfa/spfa.md -->

## Resultado de estudo

`spfaInfo` retorna `Result<SpfaInfo, AlgorithmError>` e adiciona inserções e remoções da fila, tamanho máximo da fila e as métricas compartilhadas de menor caminho. Um ciclo negativo alcançável é retornado como `Err(AlgorithmError)`.

<!-- @include: ../../../.fragments/modules/algorithm/spfa/study-result.md -->
