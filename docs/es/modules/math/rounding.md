---
title: Redondeo
description: Funciones de redondeo exportadas por tyrus:math.
---

# Redondeo

| Función | Firma | Descripción |
|---------|-------|-------------|
| [`floor`](#floor) | `(x: f64) => f64` | Redondea hacia abajo |
| [`ceil`](#ceil) | `(x: f64) => f64` | Redondea hacia arriba |
| [`trunc`](#trunc) | `(x: f64) => f64` | Trunca hacia cero |
| [`round`](#round) | `(x: f64) => f64` | Redondea al entero más cercano |
| [`roundTo`](#roundto) | `(x: f64, decimals: i32) => f64` | Redondea a N decimales |

#### `floor()`

Redondea un número hacia abajo al entero más cercano.

<SignatureBlock label="Firma">floor(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-floor.md -->

#### `ceil()`

Redondea un número hacia arriba al entero más cercano.

<SignatureBlock label="Firma">ceil(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-ceil.md -->

#### `trunc()`

Trunca un número hacia cero, eliminando la parte fraccionaria.

<SignatureBlock label="Firma">trunc(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-trunc.md -->

#### `round()`

Redondea un número al entero más cercano.

<SignatureBlock label="Firma">round(x: f64): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-round.md -->

#### `roundTo()`

Redondea un número a una cantidad especificada de decimales.

<SignatureBlock label="Firma">roundTo(x: f64, decimals: i32): f64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/math/rounding-roundTo.md -->
