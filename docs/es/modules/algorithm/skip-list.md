---
title: SkipList
description: "Skip list aleatoria (conjunto ordenado) de tyrus:algorithm."
---

# SkipList

`SkipList<T>` es un conjunto ordenado implementado sobre una skip list: una lista enlazada de nodos atravesada por carriles exprés aleatorios. Mantiene cada elemento en orden ascendente e ignora las inserciones duplicadas. Los niveles se sortean al azar hasta 16, por lo que cada búsqueda, inserción y eliminación es O(log n) en promedio, y el diseño de los carriles no requiere rebalanceo. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SkipList<T>()` | Crea una nueva skip list vacía |

`T` puede ser cualquier tipo entero, cualquier tipo de punto flotante, `boolean` o `string`.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `insert` | `(T) => boolean` | `boolean` | Agrega un elemento; retorna `true` cuando no estaba presente |
| `remove` | `(T) => boolean` | `boolean` | Elimina un elemento; retorna `true` cuando estaba presente |
| `contains` | `(T) => boolean` | `boolean` | Si el elemento está en la lista |
| `min` | `() => T?` | `T?` | Elemento más pequeño, o ningún valor si está vacío |
| `max` | `() => T?` | `T?` | Elemento más grande, o ningún valor si está vacío |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si la lista está vacía |
| `clear` | `() => void` | `void` | Elimina todos los elementos |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `insert` | O(log n) esperado |
| `remove` | O(log n) esperado |
| `contains` | O(log n) esperado |
| `min` | O(1) |
| `max` | O(n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

Los límites logarítmicos esperados se cumplen para cualquier orden de inserción, porque los carriles exprés se construyen a partir de niveles aleatorios, no del historial de actualizaciones; los extremos de la lista se encuentran recorriendo el carril inferior, así que `min` lee el primer nodo y `max` camina hasta el último.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/skip-list/complete-example.md -->

### Orden de strings

<!-- @include: ../../../.fragments/modules/algorithm/skip-list/string-ordering.md -->
