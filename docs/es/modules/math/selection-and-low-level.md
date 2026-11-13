---
title: Selección y bajo nivel
description: Funciones de selección y bajo nivel exportadas por tyrus:math.
---

# Selección y bajo nivel

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`max`](#max) | `(a: f64, b: f64) => f64` | Máximo |
| [`min`](#min) | `(a: f64, b: f64) => f64` | Mínimo |
| [`clamp`](#clamp) | `(value: f64, min: f64, max: f64) => f64` | Limita valor al intervalo |
| [`clampMin`](#clampmin) | `(value: f64, min: f64) => f64` | Garantiza valor mínimo |
| [`clampMax`](#clampmax) | `(value: f64, max: f64) => f64` | Garantiza valor máximo |
| [`imul`](#imul) | `(a: i32, b: i32) => i32` | Multiplicación C-style 32-bit |
| [`clz32`](#clz32) | `(x: i32) => i32` | Cuenta bits cero a la izquierda en entero 32-bit |

#### `max()`

Devuelve el mayor de dos números.

<SignatureBlock label="Firma">max(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-max.md -->

#### `min()`

Devuelve el menor de dos números.

<SignatureBlock label="Firma">min(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-min.md -->

#### `clamp()`

Restringe un valor a un rango dado.

<SignatureBlock label="Firma">clamp(value: f64, min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clamp.md -->

#### `clampMin()`

Garantiza un valor mínimo.

<SignatureBlock label="Firma">clampMin(value: f64, min: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clampMin.md -->

#### `clampMax()`

Garantiza un valor máximo.

<SignatureBlock label="Firma">clampMax(value: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clampMax.md -->

#### `imul()`

Realiza multiplicación entera C-style de 32 bits, truncando el overflow.

<SignatureBlock label="Firma">imul(a: i32, b: i32): i32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-imul.md -->

#### `clz32()`

Cuenta los bits cero a la izquierda en un entero de 32 bits.

<SignatureBlock label="Firma">clz32(x: i32): i32</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/selection-clz32.md -->
