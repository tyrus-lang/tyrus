---
title: Counter
description: "Estrutura de contagem de frequências de tyrus:algorithm."
---

# Counter

`Counter<T>` conta valores escalares e preserva a ordem da primeira inserção quando há empate de frequências.

## Construção

`Counter<T>()` cria um contador vazio. `T` deve ser um tipo escalar: `boolean`, `char`, inteiro ou ponto flutuante. Valores com posse no heap, como `string`, não são aceitos.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `add` | `(T, i32?) => void` | `void` | Incrementa um item por uma contagem positiva; o padrão é 1. |
| `remove` | `(T, i32?) => void` | `void` | Decrementa um item por uma contagem positiva; itens ausentes ou esgotados são removidos. |
| `count` | `(T) => i32` | `i32` | Retorna a contagem, ou 0 quando ausente. |
| `mostCommon` | `(usize) => [T, i32][]` | `[T, i32][]` | Retorna até essa quantidade de pares `[item, count]` em contagem decrescente e estável nos empates. |
| `total` | `() => i32` | `i32` | Retorna a soma de todas as contagens. |
| `unique` | `() => usize` | `usize` | Retorna o número de itens presentes. |
| `isEmpty` | `() => boolean` | `boolean` | Retorna se o contador está vazio. |
| `clear` | `() => void` | `void` | Remove todos os itens. |

`add`, `remove` e `total` encerram com erro de runtime se uma contagem positiva `i32` não puder ser representada.

## Complexidade

| Operação | Complexidade |
|-----------|--------------|
| `add`, `remove`, `count` | O(1) médio |
| `mostCommon(k)` | O(n log n) |
| `total`, `clear` | O(n) |
| `unique`, `isEmpty` | O(1) |

## Exemplo

<!-- @include: ../../../.fragments/modules/algorithm/counter/example.md -->
