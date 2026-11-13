---
title: SplayTree
description: "Árvore de splay autoadjustável de tyrus:algorithm."
---

# SplayTree

`SplayTree<T>` é um conjunto ordenado implementado sobre uma árvore de splay autoadjustável. Cada acesso move o elemento para a raiz por uma sequência de rotações, então elementos acessados recentemente ficam mais baratos de alcançar de novo. As operações são logarítmicas amortizadas. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SplayTree<T>()` | Cria uma nova árvore de splay vazia |

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
| `insert` | O(log n) amortizado |
| `contains` | O(log n) amortizado |
| `len` / `isEmpty` | O(1) |

Uma única operação pode tocar todos os nós, mas uma sequência de m operações sobre uma árvore de n elementos sempre roda em O(m log n): o custo amortizado de cada operação é O(log n), e padrões de acesso com localidade temporal são ainda mais rápidos na prática.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/splay-tree/complete-example.md -->
