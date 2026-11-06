---
title: SortedSet
description: "Estrutura de dados de conjunto ordenado de tyrus:algorithm."
---

# SortedSet

`SortedSet<T>` é um conjunto ordenado implementado sobre uma árvore binária de busca balanceada. Ele mantém cada elemento em ordem crescente e ignora inserções duplicadas. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SortedSet<T>()` | Cria um novo conjunto ordenado vazio |

`T` pode ser qualquer tipo inteiro, qualquer tipo de ponto flutuante, `boolean` ou `string`.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `insert` | `(T) => boolean` | `boolean` | Adiciona um elemento; retorna `true` quando ele não estava presente |
| `remove` | `(T) => boolean` | `boolean` | Remove um elemento; retorna `true` quando ele estava presente |
| `contains` | `(T) => boolean` | `boolean` | Se o elemento está no conjunto |
| `min` | `() => T?` | `T?` | Menor elemento, ou nenhum valor quando vazio |
| `max` | `() => T?` | `T?` | Maior elemento, ou nenhum valor quando vazio |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se o conjunto está vazio |
| `clear` | `() => void` | `void` | Remove todos os elementos |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `insert` | O(log n) |
| `remove` | O(log n) |
| `contains` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

A árvore permanece balanceada, então toda busca, inserção e remoção é logarítmica independentemente da ordem de inserção.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/sorted-set/complete-example.md -->

### Ordenação de strings

<!-- @include: ../../../.fragments/modules/algorithm/sorted-set/string-ordering.md -->
