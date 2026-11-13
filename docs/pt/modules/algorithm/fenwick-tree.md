---
title: FenwickTree
description: "Estrutura de dados árvore de Fenwick de tyrus:algorithm."
---

# FenwickTree

`FenwickTree` implementa uma árvore de Fenwick (binary indexed tree) sobre um array de tamanho fixo de inteiros de 32 bits, com suporte a somas de prefixo e atualizações pontuais em O(log n).

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `FenwickTree(n)` | Cria uma árvore sobre `n` posições inicializadas em zero |

As posições são indexadas a partir de zero, de `0` a `n - 1`. O comprimento `n` pode ser zero.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `add` | `(index: usize, delta: i32) => void` | `void` | Adiciona `delta` ao elemento em `index` |
| `sum` | `(index: usize) => i32` | `i32` | Retorna a soma dos primeiros `index` elementos, ou seja, o intervalo `[0, index)` |
| `len` | `() => usize` | `usize` | Retorna o número de posições |
| `isEmpty` | `() => boolean` | `boolean` | Se a árvore não tem posições |

`sum(index)` retorna a soma dos primeiros `index` elementos (limite superior exclusivo): `sum(0)` é `0` e `sum(n)` é o total sobre todas as posições. `add` aborta com erro de runtime quando `index >= n`; `sum` aborta quando `index > n`.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `add` | O(log n) |
| `sum` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/construction.md -->

### add

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/add.md -->

### sum

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/sum.md -->

### Deltas negativos

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/negative-deltas.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/len-and-isempty.md -->
