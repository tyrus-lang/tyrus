---
title: AvlTree
description: "Árvore AVL autobalanceada de tyrus:algorithm."
---

# AvlTree

`AvlTree<T>` é um conjunto ordenado implementado sobre uma árvore AVL autobalanceada. Cada inserção rebalanceia a árvore para que as duas subárvores de qualquer nó difiram em no máximo um nível de altura, o que garante operações logarítmicas no pior caso. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `AvlTree<T>()` | Cria uma nova árvore AVL vazia |

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

O invariante estrito de altura faz o pior caso ser tão rápido quanto o caso médio: toda operação é logarítmica independentemente da ordem de inserção.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/avl-tree/complete-example.md -->
