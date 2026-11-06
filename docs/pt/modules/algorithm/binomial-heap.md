---
title: BinomialHeap
description: "Fila de prioridade max-heap binomial de tyrus:algorithm."
---

# BinomialHeap

`BinomialHeap<T>` implementa uma fila de prioridade **max-heap** como uma floresta de árvores binomiais implementada pelo runtime nativo. Diferente de um binary heap, que mantém um único array contíguo, o binomial heap guarda seus elementos numa coleção de árvores binomiais cujos tamanhos são os dígitos binários da contagem de elementos, então unir duas filas custa O(log n) em vez de reconstruir a estrutura inteira.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `BinomialHeap<T>()` | Cria uma nova fila vazia |

O parâmetro de tipo `T` determina o tipo do elemento. `T` deve ser um tipo numérico ordenável (`i8`–`i128`, `u8`–`u128`, `f32`, `f64` ou `f128`).

## Métodos

| Método | Assinatura | Retorna | Descrição |
|--------|------------|---------|-----------|
| `push` | `(T) => void` | `void` | Insere um elemento |
| `pop` | `() => T?` | `T?` | Remove e retorna o maior elemento, ou vazio se vazio |
| `peek` | `() => T?` | `T?` | Retorna o maior elemento sem removê-lo, ou vazio se vazio |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Se a fila não tem elementos |
| `clear` | `() => void` | `void` | Remove todos os elementos |

O handle nativo é liberado automaticamente quando a variável sai de escopo.

## Complexidade

| Operação | Complexidade Temporal |
|----------|-----------------------|
| `push` | Amortizado O(1), pior caso O(log n) |
| `pop` | O(log n) |
| `peek` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

`peek()` percorre a lista de raízes, então custa O(log n) em vez do O(1) de um binary heap. `push` custa O(1) amortizado porque a maioria das inserções apenas anexa uma nova árvore à lista de raízes; um `pop` que a remove reconstrói a fila em O(log n).

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/construction.md -->

### push

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/push.md -->

### pop

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/pop.md -->

### peek

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/peek.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/binomial-heap/clear.md -->
