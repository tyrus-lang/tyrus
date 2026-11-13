---
title: suffixArray
description: "Construção de suffix array para strings de tyrus:algorithm."
---

# suffixArray

`suffixArray(text: string): usize[]` retorna o índice inicial de cada sufixo de `text`, ordenados lexicograficamente pela comparação dos bytes do sufixo. O resultado é um array novo de propriedade do chamador, com uma entrada por caractere da entrada; seu primeiro elemento é o índice inicial do menor sufixo. É uma função livre de `tyrus:algorithm`, importada como qualquer outro nome desse módulo.

## Função

| Assinatura | Retorna | Descrição |
|------------|---------|-----------|
| `suffixArray(text: string)` | `usize[]` | Índice inicial de cada sufixo, ordenado lexicograficamente |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| construção | O(n² log n), ordenação direta de sufixos |
| memória | O(n) |

## Exemplos

### banana

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/banana.md -->

### vazio e um único caractere

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/empty-and-single-character.md -->

### importação por namespace

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/namespace-import.md -->

## Resultado de estudo

<!-- @include: ../../../.fragments/modules/algorithm/suffix-array/study-result.md -->

`sortRoundCount` conta as rodadas do merge bottom-up, `comparisonCount` conta as comparações entre sufixos e `comparedByteCount` conta os pares de bytes inspecionados por essas comparações. A instrumentação mantém armazenamento adicional O(n) e o mesmo limite de tempo O(n² log n) no pior caso da ordenação direta de sufixos.
