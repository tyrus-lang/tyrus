---
title: Treap
description: "Treap aleatório de tyrus:algorithm."
---

# Treap

`Treap<T>` é um conjunto ordenado implementado sobre um treap aleatório: uma árvore binária de busca em que cada nó também carrega uma prioridade aleatória e a árvore é ordenada por heap conforme essa prioridade. A aleatoriedade mantém a árvore balanceada em expectativa para qualquer sequência de inserções. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `Treap<T>()` | Cria um novo treap vazio |

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
| `insert` | O(log n) esperado |
| `contains` | O(log n) esperado |
| `len` / `isEmpty` | O(1) |

As prioridades aleatórias mantêm a árvore balanceada em expectativa, então toda operação é logarítmica para qualquer entrada, inclusive dados pré-ordenados.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/treap/complete-example.md -->
