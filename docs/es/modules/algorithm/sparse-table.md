---
title: SparseTable
description: "Tabla RMQ inmutable de tyrus:algorithm."
---

# SparseTable

`SparseTable` precomputa una tabla inmutable de mínimo por intervalo sobre un arreglo de tamaño fijo de enteros de 32 bits, respondiendo consultas de mínimo en O(1) después de un preprocesamiento de O(n log n).

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `SparseTable(data)` | Construye una tabla RMQ sobre los valores en `data` |

La tabla se construye a partir de un arreglo no genérico de valores `i32`. Las posiciones se indexan desde cero, de `0` a `len - 1`. La longitud puede ser cero. Una vez construida, la tabla es inmutable: usa `SegmentTree` cuando necesites actualizaciones puntuales.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `query` | `(from: usize, to: usize) => i32` | `i32` | Retorna el mínimo del intervalo `[from, to)` |
| `len` | `() => usize` | `usize` | Retorna el número de posiciones |
| `isEmpty` | `() => boolean` | `boolean` | Si la tabla no tiene posiciones |

Los intervalos usan límite superior exclusivo: `query(from, to)` retorna el mínimo de los elementos en las posiciones `from` hasta `to - 1`, así que `query(0, len)` es el mínimo sobre todas las posiciones. `query` aborta con error de runtime cuando `from >= to` o `to > len`.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `query` | O(1) |
| `len` | O(1) |
| `isEmpty` | O(1) |

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/construction.md -->

### query

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/query.md -->

### len y isEmpty

<!-- @include: ../../../.fragments/modules/algorithm/sparse-table/len-and-isempty.md -->
