---
title: UnionFind
description: "Estructura de datos disjoint set (union-find) de tyrus:algorithm."
---

# UnionFind

`UnionFind<T>` implementa una estructura de datos disjoint set (union-find) con compresión de camino y unión por rango. Mantiene una colección de conjuntos no superpuestos y soporta operaciones eficientes de unión y búsqueda.

## Construcción

| Sintaxis | Descripción |
|----------|-------------|
| `UnionFind<u32>(n)` | Crea un nuevo conjunto disjunto con `n` elementos indexados de `0` a `n - 1` |
| `UnionFind<u8>(n)` | Igual, con tipo de elemento `u8` |
| `UnionFind<u16>(n)` | Igual, con tipo de elemento `u16` |
| `UnionFind<u64>(n)` | Igual, con tipo de elemento `u64` |

Solo se permiten tipos de enteros sin signo como tipo de elemento.

## Métodos

| Método | Firma | Retorna | Descripción |
|--------|-------|---------|-------------|
| `find` | `(T) => T` | `T` | Representante raíz del conjunto que contiene `x` |
| `union` | `(T, T) => boolean` | `boolean` | Fusiona conjuntos; retorna `true` si hubo fusión |
| `connected` | `(T, T) => boolean` | `boolean` | Si `x` e `y` pertenecen al mismo conjunto |
| `setCount` | `() => T` | `T` | Número de conjuntos disjuntos |

Los elementos son enteros indexados desde `0` hasta `n - 1`. El handle nativo se libera automáticamente cuando la variable sale del ámbito.

## Complejidad

| Operación | Complejidad Temporal |
|-----------|----------------------|
| `find` | Amortizado O(α(n)), prácticamente constante |
| `union` | Amortizado O(α(n)), prácticamente constante |
| `connected` | Amortizado O(α(n)), prácticamente constante |
| `setCount` | O(1) |

Donde α es la función inversa de Ackermann, que es efectivamente constante para todas las entradas prácticas.

## Ejemplos

### Construcción

<!-- @include: ../../../.fragments/modules/algorithm/union-find/construction.md -->

### find

<!-- @include: ../../../.fragments/modules/algorithm/union-find/find.md -->

### union

<!-- @include: ../../../.fragments/modules/algorithm/union-find/union.md -->

### connected

<!-- @include: ../../../.fragments/modules/algorithm/union-find/connected.md -->

### setCount

<!-- @include: ../../../.fragments/modules/algorithm/union-find/setcount.md -->

