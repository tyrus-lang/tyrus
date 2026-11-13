---
title: BinaryHeap
description: "Cola de prioridad max-heap binaria de tyrus:algorithm."
---

# BinaryHeap

`BinaryHeap<T>` implementa una cola de prioridad **max-heap** binaria implementada por el runtime nativo. Los elementos se ordenan por valor usando un comparador nativo generado por tipo de elemento; el elemento más grande siempre está en la cima.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `BinaryHeap<T>()` | Crea una nueva cola vacía con capacidad predeterminada |

El parámetro de tipo `T` determina el tipo de elemento. `T` debe ser un tipo numérico ordenable (`i8`–`i128`, `u8`–`u128`, `f32`, `f64` o `f128`).

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `push` | `(T) => void` | `void` | Inserta un elemento |
| `pop` | `() => T?` | `T?` | Elimina y retorna el elemento más grande, o vacío si está vacío |
| `peek` | `() => T?` | `T?` | Retorna el elemento más grande sin eliminarlo, o vacío si está vacío |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si la cola no tiene elementos |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `push` | O(log n) |
| `pop` | O(log n) |
| `peek` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Ejemplos

### Construcción

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

