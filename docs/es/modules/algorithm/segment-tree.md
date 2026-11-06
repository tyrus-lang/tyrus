---
title: SegmentTree
description: "Estructura de datos árbol de segmentos de tyrus:algorithm."
---

# SegmentTree

`SegmentTree` implementa un árbol de segmentos sobre un arreglo de tamaño fijo de enteros de 32 bits, con soporte de consultas por intervalo y actualizaciones puntuales en O(log n).

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SegmentTree(data)` | Crea un árbol sobre los valores en `data` |

El árbol se construye a partir de un arreglo no genérico de valores `i32`. Las posiciones se indexan desde cero, de `0` a `len - 1`. La longitud puede ser cero.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `update` | `(index: usize, value: i32) => void` | `void` | Reemplaza el elemento en `index` por `value` |
| `query` | `(from: usize, to: usize) => i32` | `i32` | Retorna la suma del intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Retorna el número de posiciones |
| `isEmpty` | `() => boolean` | `boolean` | Si el árbol no tiene posiciones |

Los intervalos usan límite superior exclusivo: `query(from, to)` suma los elementos en las posiciones `from` hasta `to - 1`, así que `query(0, len)` es el total sobre todas las posiciones y `query(k, k)` es `0`. `update` aborta con error de runtime cuando `index >= len`; `query` aborta cuando `from > to` o `to > len`.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `update` | O(log n) |
| `query` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/construction.md -->

### query

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/query.md -->

### update

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/update.md -->

### Valores negativos

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/negative-values.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/segment-tree/len-and-isempty.md -->
