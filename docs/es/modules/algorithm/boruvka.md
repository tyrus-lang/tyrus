---
title: boruvka
description: "Árbol de expansión mínima de Boruvka en tyrus:algorithm."
---

# boruvka

`boruvka(edges, vertexCount)` elige repetidamente la arista saliente más barata de cada componente. Devuelve un bosque de expansión mínima, con un árbol por cada componente conexo.

Tanto `edges` como `boruvkaInfo` también aceptan `Graph<N, i32>`; agregue ambas direcciones de cada arista no dirigida al `Graph`.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `boruvka(edges: [usize, usize, i32][], vertexCount: usize)` | `[usize, usize, i32][]` | O(E log V) |

<!-- @include: ../../../.fragments/modules/algorithm/boruvka/boruvka.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/boruvka/study-result.md -->
