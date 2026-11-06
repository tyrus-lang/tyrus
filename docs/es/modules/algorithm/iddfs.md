---
title: iddfs
description: "Búsqueda de grafos en profundidad con profundización iterativa de tyrus:algorithm."
---

# iddfs

`iddfs(adj, start, maxDepth)` repite DFS limitada por profundidad y devuelve el orden de primer descubrimiento hasta `maxDepth`. Acepta una lista de adyacencia y devuelve índices de vértices.

| Firma | Devuelve | Complejidad |
|-------|----------|-------------|
| `iddfs(adj: usize[][], start: usize, maxDepth: usize)` | `usize[]` | O(V + E) por pasada; memoria O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/iddfs/iddfs.md -->

## Resultado de estudio

`iddfsInfo` devuelve `IddfsInfo` con el orden de visita final, los límites de profundidad intentados y métricas deterministas de recorrido.

<!-- @include: ../../../.fragments/modules/algorithm/iddfs/study-result.md -->
