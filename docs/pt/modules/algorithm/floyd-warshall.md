---
title: Floyd-Warshall
description: "Menores caminhos entre todos os pares de tyrus:algorithm."
---

# Floyd-Warshall

`floydWarshall(graph)` calcula as menores distâncias entre todos os pares. A entrada pode ser uma matriz de pesos `(i32?)[][]` ou `Graph<N, i32>`; ausências representam arestas inexistentes.

| Assinatura | Retorna | Complexidade |
|------------|---------|--------------|
| `floydWarshall(matrix: (i32?)[][])` | `(i32?)[][]` | tempo O(V³), memória O(V²) |

<!-- @include: ../../../.fragments/modules/algorithm/floyd-warshall/floyd-warshall.md -->

## Resultado de estudo

`floydWarshallInfo` retorna a matriz final, o número de passagens por vértice intermediário e atualizações de distância e se a diagonal final revela um ciclo negativo.

<!-- @include: ../../../.fragments/modules/algorithm/floyd-warshall/study-result.md -->
