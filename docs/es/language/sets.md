---
title: Set
description: Colecciones Set y sus operaciones nativas.
---

# Set


## Métodos y propiedades nativos

| Miembro | Firma | Comportamiento |
| --- | --- | --- |
| `size` | `usize` | Cantidad de valores distintos. |
| `add` | `add(value: T): Set<T>` | Inserta un valor si todavía no existe y devuelve el set. |
| `has` | `has(value: T): boolean` | Comprueba si un valor está presente. |
| `delete` | `delete(value: T): boolean` | Elimina un valor e indica si estaba presente. |
| `values` | `values(): T[]` | Devuelve los valores actuales como un array. |

También se puede recorrer un set directamente con `for...of`. Añadir un valor existente no crea un duplicado.

<!-- @include: ../../.fragments/language/collections/set.md -->
