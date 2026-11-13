---
title: Deque
description: "Estrutura de dados fila de duas pontas de tyrus:algorithm."
---

# Deque

`Deque<T>` implementa uma fila de duas pontas com inserção e remoção O(1) amortizado em ambas as extremidades.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `Deque<T>()` | Cria um novo deque vazio |

O parâmetro de tipo `T` determina o tipo do elemento. `T` pode ser um valor escalar (`boolean`, `char`, inteiro ou float); valores com posse no heap, como `string`, não são aceitos.

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `pushFront` | `(T) => void` | `void` | Adiciona um elemento na frente |
| `pushBack` | `(T) => void` | `void` | Adiciona um elemento no final |
| `popFront` | `() => T?` | `T?` | Remove e retorna o elemento da frente, ou vazio se vazio |
| `popBack` | `() => T?` | `T?` | Remove e retorna o elemento do final, ou vazio se vazio |
| `peekFront` | `() => T?` | `T?` | Retorna o elemento da frente sem remover, ou vazio se vazio |
| `peekBack` | `() => T?` | `T?` | Retorna o elemento do final sem remover, ou vazio se vazio |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se o deque não tem elementos |
| `clear` | `() => void` | `void` | Remove todos os elementos |

`popFront`, `popBack`, `peekFront` e `peekBack` retornam `T?`, chame com verificação de presença ou valor padrão.

O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `pushFront` | O(1) |
| `pushBack` | Amortizado O(1) |
| `popFront` | O(1) |
| `popBack` | O(1) |
| `peekFront` | O(1) |
| `peekBack` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(1) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/deque/construction.md -->

### pushFront e pushBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/pushfront-and-pushback.md -->

### popFront

<!-- @include: ../../../.fragments/modules/algorithm/deque/popfront.md -->

### popBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/popback.md -->

### peekFront e peekBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/peekfront-and-peekback.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/deque/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/deque/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/deque/clear.md -->

### pushFront e popBack misturados (comportamento de fila)

<!-- @include: ../../../.fragments/modules/algorithm/deque/mixed-pushfront-and-popback-queue-behavior.md -->

