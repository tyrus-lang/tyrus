---
title: RedBlackTree
description: "Árvore rubro-negra autobalanceada de tyrus:algorithm."
---

# RedBlackTree

`RedBlackTree<T>` é um conjunto ordenado implementado sobre uma árvore rubro-negra autobalanceada. Os nós são coloridos de vermelho ou preto sob invariantes que mantêm todo caminho da raiz até uma folha com no máximo o dobro do comprimento de qualquer outro, o que garante operações logarítmicas no pior caso. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `RedBlackTree<T>()` | Cria uma nova árvore rubro-negra vazia |

`T` pode ser qualquer tipo inteiro, qualquer tipo de ponto flutuante, `boolean` ou `string`.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `insert` | `(T) => void` | `void` | Adiciona um elemento; duplicatas são ignoradas |
| `contains` | `(T) => boolean` | `boolean` | Se o elemento está na árvore |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se a árvore está vazia |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `insert` | O(log n) no pior caso |
| `contains` | O(log n) no pior caso |
| `len` / `isEmpty` | O(1) |

Os invariantes de coloração mantêm a árvore balanceada, então toda operação é logarítmica independentemente da ordem de inserção.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/red-black-tree/complete-example.md -->
