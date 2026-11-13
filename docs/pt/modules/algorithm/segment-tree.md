---
title: SegmentTree
description: "Estrutura de dados árvore de segmentos de tyrus:algorithm."
---

# SegmentTree

`SegmentTree` implementa uma árvore de segmentos sobre um array de tamanho fixo de inteiros de 32 bits, com suporte a consultas por intervalo e atualizações pontuais em O(log n).

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SegmentTree(data)` | Cria uma árvore sobre os valores em `data` |

A árvore é construída a partir de um array não genérico de valores `i32`. As posições são indexadas a partir de zero, de `0` a `len - 1`. O comprimento pode ser zero.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `update` | `(index: usize, value: i32) => void` | `void` | Substitui o elemento em `index` por `value` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Retorna a soma do intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Retorna o número de posições |
| `isEmpty` | `() => boolean` | `boolean` | Se a árvore não tem posições |

Os intervalos usam limite superior exclusivo: `query(from, to)` soma os elementos nas posições `from` até `to - 1`, então `query(0, len)` é o total sobre todas as posições e `query(k, k)` é `0`. `update` aborta com erro de runtime quando `index >= len`; `query` aborta quando `from > to` ou `to > len`.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `update` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/construction.md -->

### query

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/query.md -->

### update

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/update.md -->

### Valores negativos

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/negative-values.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/len-and-isempty.md -->
