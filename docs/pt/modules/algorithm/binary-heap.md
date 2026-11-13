---
title: BinaryHeap
description: "Fila de prioridade max-heap binária de tyrus:algorithm."
---

# BinaryHeap

`BinaryHeap<T>` implementa uma fila de prioridade **max-heap** binária implementada pelo runtime nativo. Os elementos são ordenados por valor usando um comparador nativo gerado por tipo de elemento; o maior elemento está sempre no topo.

## Construção

| Sintaxe | Descrição |
|---------|-----------|
| `BinaryHeap<T>()` | Cria uma nova fila vazia com capacidade padrão |

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
| `push` | O(log n) |
| `pop` | O(log n) |
| `peek` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Exemplos

### Construção

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/construction.md -->

### push

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/push.md -->

### pop

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/pop.md -->

### peek

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/peek.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/binary-heap/clear.md -->

