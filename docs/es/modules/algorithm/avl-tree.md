---
title: AvlTree
description: "Árbol AVL autoequilibrado de tyrus:algorithm."
---

# AvlTree

`AvlTree<T>` es un conjunto ordenado implementado sobre un árbol AVL autoequilibrado. Cada inserción reequilibra el árbol para que los dos subárboles de cualquier nodo difieran en a lo sumo un nivel de altura, lo que garantiza operaciones logarítmicas en el peor caso. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `AvlTree<T>()` | Crea un nuevo árbol AVL vacío |

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

El invariante estricto de altura hace que el peor caso sea tan rápido como el caso promedio: toda operación es logarítmica independientemente del orden de inserción.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/avl-tree/complete-example.md -->
