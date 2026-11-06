---
title: RedBlackTree
description: "Árbol rojo-negro autoequilibrado de tyrus:algorithm."
---

# RedBlackTree

`RedBlackTree<T>` es un conjunto ordenado implementado sobre un árbol rojo-negro autoequilibrado. Los nodos se colorean de rojo o negro bajo invariantes que mantienen todo camino desde la raíz hasta una hoja con a lo sumo el doble del largo de cualquier otro, lo que garantiza operaciones logarítmicas en el peor caso. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `RedBlackTree<T>()` | Crea un nuevo árbol rojo-negro vacío |

`T` puede ser cualquier tipo entero, cualquier tipo de punto flotante, `boolean` o `string`.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `insert` | `(T) => void` | `void` | Agrega un elemento; los duplicados se ignoran |
| `contains` | `(T) => boolean` | `boolean` | Si el elemento está en el árbol |
| `len` | `() => usize` | `usize` | Número de elementos |
| `isEmpty` | `() => boolean` | `boolean` | Si el árbol está vacío |

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `insert` | O(log n) en el peor caso |
| `contains` | O(log n) en el peor caso |
| `len` / `isEmpty` | O(1) |

Los invariantes de coloración mantienen el árbol equilibrado, por lo que toda operación es logarítmica independientemente del orden de inserción.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/red-black-tree/complete-example.md -->
