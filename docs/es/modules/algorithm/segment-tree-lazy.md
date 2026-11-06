---
title: SegmentTreeLazy
description: "Estructura de datos árbol de segmentos perezoso de tyrus:algorithm."
---

# SegmentTreeLazy

`SegmentTreeLazy` implementa un árbol de segmentos sobre un arreglo de tamaño fijo de enteros de 32 bits, con soporte para consultas por intervalo y sumas por intervalo en O(log n) mediante propagación perezosa.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SegmentTreeLazy(data)` | Crea un árbol sobre los valores de `data` |

El árbol se construye a partir de un arreglo no genérico de valores `i32`. Las posiciones se indexan desde cero, de `0` a `len - 1`. La longitud puede ser cero.

## Métodos

| Método | Firma | Devuelve | Descripción |
|--------|-------|----------|-------------|
| `updateRange` | `(from: usize, to: usize, delta: i32) => void` | `void` | Suma `delta` a cada elemento de `[from, to)` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Devuelve la suma del intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Devuelve el número de posiciones |
| `isEmpty` | `() => boolean` | `boolean` | Indica si el árbol no tiene posiciones |

Los intervalos usan un límite superior exclusivo: `query(from, to)` suma los elementos de las posiciones desde `from` hasta `to - 1`, y `updateRange(from, to, delta)` suma `delta` a esas mismas posiciones. `query(k, k)` devuelve `0` y `updateRange(k, k, delta)` no tiene efecto. Ambos métodos de intervalo abortan con un error de runtime cuando `from > to` o `to > len`.

## Complejidad

| Operación | Complejidad temporal |
|-----------|----------------------|
| `updateRange` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Ejemplos

### Construcción y query

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/construction-and-query.md -->

### updateRange

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/updaterange.md -->

### Deltas negativos y árboles vacíos

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree-lazy/negative-deltas-and-empty-trees.md -->
