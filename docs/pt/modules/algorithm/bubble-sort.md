---
title: Bubble sort
description: "Bubble Sort direto e um resultado de estudo com métricas determinísticas."
---

# Bubble sort

O módulo oferece duas funções importadas. `bubbleSort(values)` retorna uma cópia crescente para uso direto, enquanto `bubbleSortInfo(values)` retorna snapshots independentes da entrada e da saída e contadores semânticos para estudar o algoritmo. Nenhuma função altera o array de entrada.

## Execução direta

| Assinatura | Retorno | Complexidade |
|------------|---------|--------------|
| `bubbleSort(values: i32[])` | `i32[]` | O(n²) no pior caso e no caso médio, O(n) no melhor caso |

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/direct-execution.md -->

## Resultado de estudo

| Export | Tipo | Finalidade |
|--------|------|------------|
| `bubbleSortInfo(values: i32[])` | Função | Retorna um `BubbleSortInfo` sem alterar `values` |
| `BubbleSortInfo` | Interface de dados | Mantém os snapshots e todas as métricas do Bubble Sort |

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/bubble-sort-info.md -->

`BubbleSortInfo` expõe `originalValues: i32[]`, `sortedValues: i32[]`, `comparisonCount: usize`, `swapCount: usize` e `passCount: usize`. Os arrays são snapshots independentes da entrada e da saída ordenada.

<!-- @include: ../../../.fragments/modules/algorithm/bubble-sort/study-result.md -->

## Definições das métricas

- `comparisonCount` incrementa uma vez para cada par adjacente avaliado pelo laço interno.
- `swapCount` incrementa uma vez quando uma comparação adjacente troca dois valores.
- `passCount` incrementa após cada percurso concluído pelo prefixo ainda não ordenado, incluindo o percurso final que não faz troca e encerra antecipadamente.
- Entradas vazias ou com um elemento executam zero comparações, trocas e passagens.

O Bubble Sort é estável porque valores iguais nunca são trocados. A variante instrumentada tem o mesmo tempo O(n²) no pior caso e O(n) no melhor caso da função direta, além de O(n) de armazenamento no resultado para os snapshots original e ordenado. Características estáticas como estabilidade e complexidade permanecem nesta documentação em vez de serem repetidas em cada resultado. A API atual aceita intencionalmente apenas `i32[]`.
