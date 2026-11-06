---
title: Basics and signs
description: Basic math functions exported by tyrus:math.
---

# Basics and signs

| Function | Signature | Description |
|----------|-----------|-------------|
| [`abs`](#abs) `(x: f64) => f64` | Absolute value |
| [`sign`](#sign) `(x: f64) => f64` | Sign (`1`, `-1`, or `0`) |
| [`sqrt`](#sqrt) `(x: f64) => f64` | Square root |
| [`cbrt`](#cbrt) `(x: f64) => f64` | Cube root |

#### `abs()`

Returns the absolute value of a number.

<SignatureBlock label="Signature">abs(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/basics-abs.md -->

#### `sign()`

Returns the sign of a number: `1.0` for positive, `-1.0` for negative, or `0.0` for zero.

<SignatureBlock label="Signature">sign(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/basics-sign.md -->

#### `sqrt()`

Returns the square root of a number. Negative inputs panic at runtime.

<SignatureBlock label="Signature">sqrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/basics-sqrt.md -->

#### `cbrt()`

Returns the cube root of a number. Negative inputs are valid.

<SignatureBlock label="Signature">cbrt(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/basics-cbrt.md -->
