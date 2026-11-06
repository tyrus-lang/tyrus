---
title: DoubleLinkedList
description: "Estrutura de dados de lista duplamente encadeada do tyrus:algorithm."
---

# DoubleLinkedList

`DoubleLinkedList<T>` implementa uma lista duplamente encadeada com inserção e remoção O(1) em ambas as extremidades.

## Construção

| Sintaxe | Descrição |
|--------|-------------|
| `DoubleLinkedList<T>()` | Cria uma nova lista duplamente encadeada vazia |
| `DoubleLinkedList.fromArray(arr)` | Cria uma lista duplamente encadeada com os elementos de `arr` em ordem |

O parâmetro de tipo `T` determina o tipo do elemento. `T` pode ser um tipo de valor escalar (`boolean`, `char`, inteiro ou float); valores possuídos pelo heap, como `string`, não são suportados.

## Métodos

| Método | Assinatura | Retorno | Descrição |
|--------|-----------|---------|-------------|
| `pushFront` | `(T) => void` | `void` | Adiciona um elemento à frente |
| `pushBack` | `(T) => void` | `void` | Adiciona um elemento ao final |
| `popFront` | `() => T?` | `T?` | Remove e retorna o elemento da frente, ou vazio se vazia |
| `popBack` | `() => T?` | `T?` | Remove e retorna o elemento do final, ou vazio se vazia |
| `front` | `() => T?` | `T?` | Retorna o elemento da frente sem removê-lo, ou vazio se vazia |
| `back` | `() => T?` | `T?` | Retorna o elemento do final sem removê-lo, ou vazio se vazia |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se a lista não tem elementos |
| `clear` | `() => void` | `void` | Remove todos os elementos |

`popFront`, `popBack`, `front` e `back` retornam `T?`, chame com uma checagem de presença ou valor padrão.

## Complexidade

| Operação | Complexidade de Tempo |
|-----------|-----------------|
| `pushFront` | O(1) |
| `pushBack` | O(1) |
| `popFront` | O(1) |
| `popBack` | O(1) |
| `front` | O(1) |
| `back` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/construction.md -->

### pushFront e pushBack

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/pushfront-and-pushback.md -->

### front e back

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/front-and-back.md -->

### popFront

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/popfront.md -->

### popBack

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/popback.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/clear.md -->

### fromArray

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/fromarray.md -->
