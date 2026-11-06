---
title: Rounding
description: Rounding functions exported by tyrus:math.
---

# Rounding

| Function | Signature | Description |
|----------|-----------|-------------|
| [`floor`](#floor) `(x: f64) => f64` | Round down |
| [`ceil`](#ceil) `(x: f64) => f64` | Round up |
| [`trunc`](#trunc) `(x: f64) => f64` | Truncate toward zero |
| [`round`](#round) `(x: f64) => f64` | Round to nearest integer |
| [`roundTo`](#roundto) `(x: f64, decimals: i32) => f64` | Round to N decimal places |

#### `floor()`

Rounds a number down to the nearest integer.

<SignatureBlock label="Signature">floor(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/rounding-floor.md -->

#### `ceil()`

Rounds a number up to the nearest integer.

<SignatureBlock label="Signature">ceil(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/rounding-ceil.md -->

#### `trunc()`

Truncates a number toward zero, removing the fractional part.

<SignatureBlock label="Signature">trunc(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/rounding-trunc.md -->

#### `round()`

Rounds a number to the nearest integer.

<SignatureBlock label="Signature">round(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/rounding-round.md -->

#### `roundTo()`

Rounds a number to a specified number of decimal places.

<SignatureBlock label="Signature">roundTo(x: f64, decimals: i32): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/rounding-roundTo.md -->
