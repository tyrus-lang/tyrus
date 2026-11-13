---
title: suffixTreeUkkonen
description: "Construção de suffix tree para strings de tyrus:algorithm."
---

# suffixTreeUkkonen

`suffixTreeUkkonen(text: string): SuffixTree` constrói uma suffix tree sobre `text` usando o algoritmo linear de Ukkonen. A `SuffixTree` retornada é um handle de propriedade do chamador de `tyrus:algorithm`; ela expõe `getSuffixArray()`, que devolve o índice inicial de cada sufixo de `text`, ordenados lexicograficamente pela comparação dos bytes do sufixo, como um array novo de propriedade do chamador com uma entrada por caractere da entrada.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `suffixTreeUkkonen(text: string)` | Construir uma suffix tree sobre `text` |

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `getSuffixArray` | `() => usize[]` | `usize[]` | Índice inicial de cada sufixo, ordenados lexicograficamente |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| construção | O(n), algoritmo de Ukkonen |
| `getSuffixArray` | O(n), uma única travessia da árvore |
| memória | O(n) |

## Exemplos

### banana

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/banana.md -->

### tipo anotado

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/annotated-type.md -->

### vazio e um único caractere

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/empty-and-single-character.md -->

### importação por namespace

<!-- @include: ../../../.fragments/modules/algorithm/suffix-tree/namespace-import.md -->
