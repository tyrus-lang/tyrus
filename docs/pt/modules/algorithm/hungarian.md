---
title: hungarian
description: "Atribuição de custo mínimo em tyrus:algorithm."
---

# hungarian

`hungarian(costMatrix)` atribui linhas a colunas distintas com custo total mínimo. A entrada é uma matriz de custos `i32[][]` e o resultado contém o custo ótimo e os pares linha-coluna.

| Assinatura | Retorno | Complexidade |
|-----------|---------|------------|
| `hungarian(costMatrix: i32[][])` | `[i32, [usize, usize][]]` | O(n³) para matriz n × n |

Para matrizes retangulares, cada linha é atribuída quando há colunas suficientes. Custos positivos e negativos são aceitos.

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/hungarian/example.md -->

## Resultado para estudo

<!-- @include: ../../../.fragments/modules/algorithm/hungarian/study-result.md -->
