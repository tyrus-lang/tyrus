---
title: Quickselect
description: "Seleciona o k-ésimo menor valor em um array de inteiros."
---

# Quickselect

Seleciona o valor na posição zero-based `k` da ordem crescente sem ordenar o array inteiro. A entrada é um `i32[]`; o resultado é `i32?` e fica ausente quando `k` está fora do array.

<!-- @include: ../../../.fragments/modules/algorithm/quickselect/quickselect.md -->

## Resultado de estudo

`quickselectInfo` retorna um `SelectionInfo` com snapshot da entrada original e métricas determinísticas de partições, comparações e trocas.

<!-- @include: ../../../.fragments/modules/algorithm/quickselect/study-result.md -->

A complexidade média é O(n), com O(n²) no pior caso e espaço auxiliar O(1).
