---
title: Bellman-Ford
description: "Rutas más cortas de Bellman-Ford de tyrus:algorithm."
---

# Bellman-Ford

`bellmanFord(graph, start)` devuelve distancias y predecesores y admite pesos negativos. Un ciclo negativo devuelve `Err(AlgorithmError)` con el código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propágalo con `?` o manéjalo con `match`.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `bellmanFord(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | tiempo O(VE), memoria O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/bellman-ford/bellman-ford.md -->

## Resultado de estudio

`bellmanFordInfo` devuelve las métricas de todas las pasadas y relajaciones e informa un ciclo negativo alcanzable como dato, incluida la primera arista que todavía permite relajación.

<!-- @include: ../../../.fragments/modules/algorithm/bellman-ford/study-result.md -->
