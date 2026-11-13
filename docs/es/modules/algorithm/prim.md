---
title: prim
description: "Árbol de expansión mínima de Prim en tyrus:algorithm."
---

# prim

`prim(adjacency, origin)` construye un árbol de expansión mínima desde `origin` usando una cola de prioridad. En entradas desconectadas, devuelve el componente alcanzable desde `origin`.

Tanto `adjacency` como `primInfo` también aceptan `Graph<N, i32>`; agregue ambas direcciones de cada arista no dirigida al `Graph`.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `prim(adjacency: [usize, i32][][], origin: usize)` | `[usize, usize, i32][]` | O((V + E) log V) |

<!-- @include: ../../../.fragments/modules/algorithm/prim/prim.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/prim/study-result.md -->
