---
title: FenwickTree
description: "Estructura de datos árbol de Fenwick de tyrus:algorithm."
---

# FenwickTree

`FenwickTree` implementa un árbol de Fenwick (binary indexed tree) sobre un arreglo de tamaño fijo de enteros de 32 bits, con soporte de sumas de prefijo y actualizaciones puntuales en O(log n).

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `FenwickTree(n)` | Crea un árbol sobre `n` posiciones inicializadas en cero |

Las posiciones se indexan desde cero, de `0` a `n - 1`. La longitud `n` puede ser cero.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `add` | `(index: usize, delta: i32) => void` | `void` | Agrega `delta` al elemento en `index` |
| `sum` | `(index: usize) => i32` | `i32` | Retorna la suma de los primeros `index` elementos, es decir, el intervalo `[0, index)` |
| `len` | `() => usize` | `usize` | Retorna el número de posiciones |
| `isEmpty` | `() => boolean` | `boolean` | Si el árbol no tiene posiciones |

`sum(index)` retorna la suma de los primeros `index` elementos (límite superior exclusivo): `sum(0)` es `0` y `sum(n)` es el total sobre todas las posiciones. `add` aborta con error de runtime cuando `index >= n`; `sum` aborta cuando `index > n`.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `add` | O(log n) |
| `sum` | O(log n) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/construction.md -->

### add

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/add.md -->

### sum

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/sum.md -->

### Deltas negativos

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/negative-deltas.md -->

### len e isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/fenwick-tree/len-and-isempty.md -->
