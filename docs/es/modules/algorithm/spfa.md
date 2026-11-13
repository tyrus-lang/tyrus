---
title: SPFA
description: "Rutas más cortas basadas en cola de tyrus:algorithm."
---

# SPFA

`spfa(graph, start)` calcula rutas más cortas desde un origen con pesos negativos y devuelve distancias y predecesores. Un ciclo negativo devuelve `Err(AlgorithmError)` con el código `ERR_ALGORITHM_NEGATIVE_CYCLE`; propágalo con `?` o manéjalo con `match`.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `spfa(adj: [usize, i32][][], start: usize)` | `Result<[i32[], usize?[]], AlgorithmError>` | peor caso O(VE), memoria O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/spfa/spfa.md -->

## Resultado de estudio

`spfaInfo` devuelve `Result<SpfaInfo, AlgorithmError>` y agrega inserciones y extracciones de la cola, tamaño máximo de la cola y las métricas compartidas de camino mínimo. Un ciclo negativo alcanzable se devuelve como `Err(AlgorithmError)`.

<!-- @include: ../../../.fragments/modules/algorithm/spfa/study-result.md -->
