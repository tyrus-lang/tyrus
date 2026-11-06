---
title: LinkedList
description: "Estructura de datos de lista enlazada simple de tyrus:algorithm."
---

# LinkedList

`LinkedList<T>` implementa una lista enlazada simple con inserción y eliminación O(1) al frente.

## Construcción

| Sintaxis | Descripción |
|--------|-------------|
| `LinkedList<T>()` | Crea una nueva lista enlazada vacía |
| `LinkedList.fromArray(arr)` | Crea una lista enlazada con los elementos de `arr` en orden |

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
| `pushBack` | O(n) |
| `popFront` | O(1) |
| `popBack` | O(n) |
| `front` | O(1) |
| `back` | O(n) |
| `len` | O(1) |
| `isEmpty` | O(1) |
| `clear` | O(n) |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/construction.md -->

### pushFront y pushBack

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/pushfront-and-pushback.md -->

### front y back

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/front-and-back.md -->

### popFront

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/popfront.md -->

### popBack

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/popback.md -->

### len

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/len.md -->

### isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/isempty.md -->

### clear

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/clear.md -->

### fromArray

<!-- @include: ../../../.fragments/modules/algorithm/linked-list/fromarray.md -->
