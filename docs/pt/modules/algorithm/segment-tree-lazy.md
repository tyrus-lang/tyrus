---
title: SegmentTreeLazy
description: "Estrutura de dados árvore de segmentos preguiçosa de tyrus:algorithm."
---

# SegmentTreeLazy

`SegmentTreeLazy` implementa uma árvore de segmentos sobre um array de tamanho fixo de inteiros de 32 bits, com suporte a consultas por intervalo e adições por intervalo em O(log n) por meio de propagação preguiçosa.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SegmentTreeLazy(data)` | Cria uma árvore sobre os valores em `data` |

A árvore é construída a partir de um array não genérico de valores `i32`. As posições são indexadas a partir de zero, de `0` a `len - 1`. O comprimento pode ser zero.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `updateRange` | `(from: usize, to: usize, delta: i32) => void` | `void` | Adiciona `delta` a cada elemento em `[from, to)` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Retorna a soma do intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Retorna o número de posições |
| `isEmpty` | `() => boolean` | `boolean` | Informa se a árvore não possui posições |

Os intervalos usam limite superior exclusivo: `query(from, to)` soma os elementos das posições de `from` até `to - 1`, e `updateRange(from, to, delta)` adiciona `delta` às mesmas posições. `query(k, k)` retorna `0` e `updateRange(k, k, delta)` não tem efeito. Os dois métodos de intervalo abortam com um erro de runtime quando `from > to` ou `to > len`.

## Complexidade

| Operação | Complexidade de tempo |
|----------|-----------------------|
| `updateRange` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Exemplos

### Construção e query

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/construction-and-query.md -->

### updateRange

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/updaterange.md -->

### Deltas negativos e árvores vazias

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/negative-deltas-and-empty-trees.md -->
