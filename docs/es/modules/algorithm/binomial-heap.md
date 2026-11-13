---
title: BinomialHeap
description: "Cola de prioridad max-heap binomial de tyrus:algorithm."
---

# BinomialHeap

`BinomialHeap<T>` implementa una cola de prioridad **max-heap** como un bosque de árboles binomiales implementado por el runtime nativo. A diferencia de un binary heap, que mantiene un único array contiguo, el binomial heap guarda sus elementos en una colección de árboles binomiales cuyos tamaños son los dígitos binarios del recuento de elementos, por lo que fusionar dos colas cuesta O(log n) en lugar de reconstruir toda la estructura.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `BinomialHeap<T>()` | Crea una nueva cola vacía |

El parámetro de tipo `T` determina el tipo del elemento. `T` debe ser un tipo numérico ordenable (`i8`–`i128`, `u8`–`u128`, `f32`, `f64` o `f128`).

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `push` | `(T) => void` | `void` | Inserta un elemento |
| `pop` | `() => T?` | `T?` | Elimina y devuelve el elemento más grande, o vacío si está vacía |
| `peek` | `() => T?` | `T?` | Devuelve el elemento más grande sin eliminarlo, o vacío si está vacía |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si la cola no tiene elementos |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `push` | Amortizado O(1), peor caso O(log n) |
| `pop` | O(log n) |
| `peek` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

`peek()` recorre la lista de raíces, por lo que cuesta O(log n) en lugar del O(1) de un binary heap. `push` cuesta O(1) amortizado porque la mayoría de las inserciones solo añaden un árbol nuevo a la lista de raíces; un `pop` que lo elimina reconstruye la cola en O(log n).

## Ejemplos

### Construcción

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
