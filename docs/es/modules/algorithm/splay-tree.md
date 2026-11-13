---
title: SplayTree
description: "Árbol de splay autoadaptativo de tyrus:algorithm."
---

# SplayTree

`SplayTree<T>` es un conjunto ordenado implementado sobre un árbol de splay autoadaptativo. Cada acceso mueve el elemento a la raíz mediante una secuencia de rotaciones, por lo que los elementos recién accedidos son más baratos de alcanzar de nuevo. Las operaciones son logarítmicas amortizadas. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SplayTree<T>()` | Crea un nuevo árbol de splay vacío |

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
| `insert` | O(log n) amortizado |
| `contains` | O(log n) amortizado |
| `len` / `isEmpty` | O(1) |

Una sola operación puede tocar todos los nodos, pero una secuencia de m operaciones sobre un árbol de n elementos siempre se ejecuta en O(m log n): el costo amortizado de cada operación es O(log n), y los patrones de acceso con localidad temporal son incluso más rápidos en la práctica.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/splay-tree/complete-example.md -->
