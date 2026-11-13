---
title: Deque
description: "Estructura de datos cola de doble extremo de tyrus:algorithm."
---

# Deque

`Deque<T>` implementa una cola de doble extremo con inserción y eliminación O(1) amortizada en ambos extremos.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `Deque<T>()` | Crea un nuevo deque vacío |

El parámetro de tipo `T` determina el tipo del elemento. `T` puede ser un valor escalar (`boolean`, `char`, entero o float); los valores con propiedad en el heap, como `string`, no son compatibles.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `pushFront` | `(T) => void` | `void` | Agrega un elemento al frente |
| `pushBack` | `(T) => void` | `void` | Agrega un elemento al final |
| `popFront` | `() => T?` | `T?` | Elimina y retorna el elemento del frente, o vacío si está vacío |
| `popBack` | `() => T?` | `T?` | Elimina y retorna el elemento del final, o vacío si está vacío |
| `peekFront` | `() => T?` | `T?` | Retorna el elemento del frente sin eliminar, o vacío si está vacío |
| `peekBack` | `() => T?` | `T?` | Retorna el elemento del final sin eliminar, o vacío si está vacío |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si el deque no tiene elementos |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

`popFront`, `popBack`, `peekFront` y `peekBack` retornan `T?`, llame con verificación de presencia o valor por defecto.

El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `pushFront` | O(1) |
| `pushBack` | Amortizado O(1) |
| `popFront` | O(1) |
| `popBack` | O(1) |
| `peekFront` | O(1) |
| `peekBack` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(1) |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/deque/construction.md -->

### pushFront y pushBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/pushfront-and-pushback.md -->

### popFront

<!-- @include: ../../../.fragments/modules/algorithm/deque/popfront.md -->

### popBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/popback.md -->

### peekFront y peekBack

<!-- @include: ../../../.fragments/modules/algorithm/deque/peekfront-and-peekback.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/deque/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/deque/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/deque/clear.md -->

### pushFront y popBack mixtos (comportamiento de cola)

<!-- @include: ../../../.fragments/modules/algorithm/deque/mixed-pushfront-and-popback-queue-behavior.md -->

