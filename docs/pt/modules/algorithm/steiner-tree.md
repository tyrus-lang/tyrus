---
title: steinerTree
description: "Árvore de conexão de terminais ponderada em tyrus:algorithm."
---

# steinerTree

`steinerTree(edges, vertexCount, terminals)` conecta os terminais solicitados por caminhos não direcionados ponderados e retorna as arestas selecionadas. Uma lista de terminais vazia retorna um array vazio; terminais inalcançáveis também retornam um array vazio.

Tanto `edges` quanto `steinerTreeInfo` também aceitam `Graph<N, i32>`; adicione as duas direções de cada aresta não direcionada ao `Graph`.

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `steinerTree(edges: [usize, usize, i32][], vertexCount: usize, terminals: usize[])` | `[usize, usize, i32][]` | O(T · (E + V) log V + E log V) |

<!-- @include: ../../../.fragments/modules/algorithm/steiner-tree/steinertree.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/steiner-tree/study-result.md -->
