---
title: SortedSet
description: "Estructura de datos de conjunto ordenado de tyrus:algorithm."
---

# SortedSet

`SortedSet<T>` es un conjunto ordenado implementado sobre un árbol binario de búsqueda balanceado. Mantiene cada elemento en orden ascendente e ignora las inserciones duplicadas. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SortedSet<T>()` | Crea un nuevo conjunto ordenado vacío |

`T` puede ser cualquier tipo entero, cualquier tipo de punto flotante, `boolean` o `string`.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `insert` | `(T) => boolean` | `boolean` | Agrega un elemento; retorna `true` cuando no estaba presente |
| `remove` | `(T) => boolean` | `boolean` | Elimina un elemento; retorna `true` cuando estaba presente |
| `contains` | `(T) => boolean` | `boolean` | Si el elemento está en el conjunto |
| `min` | `() => T?` | `T?` | Elemento más pequeño, o ningún valor si está vacío |
| `max` | `() => T?` | `T?` | Elemento más grande, o ningún valor si está vacío |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si el conjunto está vacío |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `insert` | O(log n) |
| `remove` | O(log n) |
| `contains` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

El árbol permanece balanceado, por lo que toda búsqueda, inserción y eliminación es logarítmica independientemente del orden de inserción.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/sorted-set/complete-example.md -->

### Orden de strings

<!-- @include: ../../../.fragments/modules/algorithm/sorted-set/string-ordering.md -->
