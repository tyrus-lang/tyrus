---
title: steinerTree
description: "Árbol de conexión ponderado de terminales en tyrus:algorithm."
---

# steinerTree

`steinerTree(edges, vertexCount, terminals)` conecta los terminales solicitados mediante caminos no dirigidos ponderados y devuelve las aristas seleccionadas. Una lista de terminales vacía devuelve un array vacío; los terminales inalcanzables también devuelven un array vacío.

Tanto `edges` como `steinerTreeInfo` también aceptan `Graph<N, i32>`; agregue ambas direcciones de cada arista no dirigida al `Graph`.

| Firma | Retorno | Complejidad |
|-------|---------|-------------|
| `steinerTree(edges: [usize, usize, i32][], vertexCount: usize, terminals: usize[])` | `[usize, usize, i32][]` | O(T · (E + V) log V + E log V) |

<!-- @include: ../../../.fragments/modules/algorithm/steiner-tree/steinertree.md -->

## Resultado de estudio

<!-- @include: ../../../.fragments/modules/algorithm/steiner-tree/study-result.md -->
