---
title: Treap
description: "Treap aleatorio de tyrus:algorithm."
---

# Treap

`Treap<T>` es un conjunto ordenado implementado sobre un treap aleatorio: un árbol binario de búsqueda en el que cada nodo también lleva una prioridad aleatoria y el árbol está ordenado por montículo según esa prioridad. La aleatoriedad mantiene el árbol equilibrado en expectativa para cualquier secuencia de inserciones. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `Treap<T>()` | Crea un nuevo treap vacío |

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
| `insert` | O(log n) esperado |
| `contains` | O(log n) esperado |
| `len` / `isEmpty` | O(1) |

Las prioridades aleatorias mantienen el árbol equilibrado en expectativa, por lo que toda operación es logarítmica para cualquier entrada, incluidos los datos preordenados.

## Ejemplos

### Ejemplo completo

<!-- @include: ../../../.fragments/modules/algorithm/treap/complete-example.md -->
