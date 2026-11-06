---
title: SortedMap
description: "Mapa ordenado de clave-valor de tyrus:algorithm."
---

# SortedMap

`SortedMap<K, V>` es un mapa ordenado de clave → valor implementado sobre un árbol binario de búsqueda balanceado. Mantiene cada entrada ordenada por clave. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SortedMap<K, V>()` | Crea un nuevo mapa ordenado vacío |

`K` puede ser cualquier tipo entero, cualquier tipo de punto flotante, `boolean` o `string`. `V` puede ser cualquier tipo.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `set` | `(K, V) => void` | `void` | Inserta o reemplaza la entrada para `key` |
| `get` | `(K) => V?` | `V?` | Valor para `key`, o ningún valor si está ausente |
| `has` | `(K) => boolean` | `boolean` | Si `key` está presente |
| `delete` | `(K) => boolean` | `boolean` | Elimina la entrada para `key`; retorna `true` cuando estaba presente |
| `min` | `() => K?` | `K?` | Clave más pequeña, o ningún valor si está vacío |
| `max` | `() => K?` | `K?` | Clave más grande, o ningún valor si está vacío |
| `len` | `() => usize` | `usize` | Número de entradas |
| `isEmpty` | `() => boolean` | `boolean` | Si el mapa está vacío |
| `clear` | `() => void` | `void` | Elimina todas las entradas |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `set` | O(log n) |
| `get` / `has` | O(log n) |
| `delete` | O(log n) |
| `min` / `max` | O(log n) |
| `len` / `isEmpty` | O(1) |
| `clear` | O(n) |

El árbol permanece balanceado, por lo que toda búsqueda, inserción y eliminación es logarítmica independientemente del orden de inserción.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/sorted-map/complete-example.md -->

### Claves de string

<!-- @include: ../../../.fragments/modules/algorithm/sorted-map/string-keys.md -->
