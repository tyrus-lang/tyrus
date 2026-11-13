---
title: SkipList
description: "Skip list aleatória (conjunto ordenado) de tyrus:algorithm."
---

# SkipList

`SkipList<T>` é um conjunto ordenado implementado sobre uma skip list: uma lista encadeada de nós atravessados por pistas expressas aleatórias. Ele mantém cada elemento em ordem crescente e ignora inserções duplicadas. Os níveis são sorteados aleatoriamente até 16, então cada busca, inserção e remoção é O(log n) em média, e o layout das pistas não exige rebalanceamento. O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `SkipList<T>()` | Cria uma nova skip list vazia |

`T` pode ser qualquer tipo inteiro, qualquer tipo de ponto flutuante, `boolean` ou `string`.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `insert` | `(T) => boolean` | `boolean` | Adiciona um elemento; retorna `true` quando ele não estava presente |
| `remove` | `(T) => boolean` | `boolean` | Remove um elemento; retorna `true` quando ele estava presente |
| `contains` | `(T) => boolean` | `boolean` | Se o elemento está na lista |
| `min` | `() => T?` | `T?` | Menor elemento, ou nenhum valor quando vazio |
| `max` | `() => T?` | `T?` | Maior elemento, ou nenhum valor quando vazio |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se a lista está vazia |
| `clear` | `() => void` | `void` | Remove todos os elementos |

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `insert` | O(log n) esperado |
| `remove` | O(log n) esperado |
| `contains` | O(log n) esperado |
| `min` | O(1) |
| `max` | O(n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

Os limites logarítmicos esperados valem para qualquer ordem de inserção, porque as pistas expressas são construídas a partir de níveis aleatórios, e não do histórico de atualizações; os extremos da lista são encontrados percorrendo a pista inferior, então `min` lê o primeiro nó e `max` caminha até o último.

## Exemplos

### Exemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/skip-list/complete-example.md -->

### Ordenação de strings

<!-- @include: ../../../.fragments/modules/algorithm/skip-list/string-ordering.md -->
