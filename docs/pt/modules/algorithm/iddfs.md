---
title: iddfs
description: "Busca de grafos em profundidade com aprofundamento iterativo de tyrus:algorithm."
---

# iddfs

`iddfs(adj, start, maxDepth)` repete DFS limitada por profundidade e retorna a ordem da primeira descoberta até `maxDepth`. Aceita uma lista de adjacência e retorna índices de vértices.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `iddfs(adj: usize[][], start: usize, maxDepth: usize)` | `usize[]` | O(V + E) por passagem; memória O(V) |

<!-- @include: ../../../.fragments/modules/algorithm/iddfs/iddfs.md -->

## Resultado de estudo

`iddfsInfo` retorna `IddfsInfo` com a ordem de visita final, os limites de profundidade tentados e métricas determinísticas de travessia.

<!-- @include: ../../../.fragments/modules/algorithm/iddfs/study-result.md -->
