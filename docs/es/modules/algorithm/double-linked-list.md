---
title: DoubleLinkedList
description: "Estructura de datos de lista doblemente enlazada de tyrus:algorithm."
---

# DoubleLinkedList

`DoubleLinkedList<T>` implementa una lista doblemente enlazada con inserción y eliminación O(1) en ambos extremos.

## Construcción

| Sintaxis | Descripción |
|--------|-------------|
| `DoubleLinkedList<T>()` | Crea una nueva lista doblemente enlazada vacía |
| `DoubleLinkedList.fromArray(arr)` | Crea una lista doblemente enlazada con los elementos de `arr` en orden |

El parámetro de tipo `T` determina el tipo de elemento. `T` puede ser un tipo de valor escalar (`boolean`, `char`, entero o float); los valores propiedad del heap, como `string`, no son compatibles.

## Métodos

| Método | Firma | Retorno | Descripción |
|--------|-----------|---------|-------------|
| `pushFront` | `(T) => void` | `void` | Agrega un elemento al frente |
| `pushBack` | `(T) => void` | `void` | Agrega un elemento al final |
| `popFront` | `() => T?` | `T?` | Elimina y devuelve el elemento frontal, o vacío si está vacía |
| `popBack` | `() => T?` | `T?` | Elimina y devuelve el elemento final, o vacío si está vacía |
| `front` | `() => T?` | `T?` | Devuelve el elemento frontal sin eliminarlo, o vacío si está vacía |
| `back` | `() => T?` | `T?` | Devuelve el elemento final sin eliminarlo, o vacío si está vacía |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si la lista no tiene elementos |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

`popFront`, `popBack`, `front` y `back` devuelven `T?`, llame con una comprobación de presencia o un valor por defecto.

## Complejidad

| Operación | Complejidad de Tiempo |
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

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/construction.md -->

### pushFront y pushBack

<!-- @include: ../../../.fragments/modules/algorithm/double-linked-list/pushfront-and-pushback.md -->

### front y back

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
