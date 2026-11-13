---
title: Trigonometry
description: Trigonometric functions exported by tyrus:math.
---

# Trigonometry

| Function | Signature | Description |
|----------|-----------|-------------|
| [`sin`](#sin) | `(x: f64) => f64` | Sine |
| [`cos`](#cos) | `(x: f64) => f64` | Cosine |
| [`tan`](#tan) | `(x: f64) => f64` | Tangent |
| [`asin`](#asin) | `(x: f64) => f64` | Arc sine |
| [`acos`](#acos) | `(x: f64) => f64` | Arc cosine |
| [`atan`](#atan) | `(x: f64) => f64` | Arc tangent |
| [`atan2`](#atan2) | `(y: f64, x: f64) => f64` | Arc tangent of `y/x` |
| [`degToRad`](#degtorad) | `(degrees: f64) => f64` | Convert degrees to radians |
| [`radToDeg`](#radtodeg) | `(radians: f64) => f64` | Convert radians to degrees |

#### `sin()`

Returns the sine of an angle in radians.

<SignatureBlock label="Signature">sin(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-sin.md -->

#### `cos()`

Returns the cosine of an angle in radians.

<SignatureBlock label="Signature">cos(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-cos.md -->

#### `tan()`

Returns the tangent of an angle in radians.

<SignatureBlock label="Signature">tan(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-tan.md -->

#### `asin()`

Returns the arc sine (inverse sine) of a value in radians.

<SignatureBlock label="Signature">asin(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-asin.md -->

#### `acos()`

Returns the arc cosine (inverse cosine) of a value in radians.

<SignatureBlock label="Signature">acos(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-acos.md -->

#### `atan()`

Returns the arc tangent (inverse tangent) of a value in radians.

<SignatureBlock label="Signature">atan(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-atan.md -->

#### `atan2()`

Returns the arc tangent of `y/x`, handling the quadrant correctly.

<SignatureBlock label="Signature">atan2(y: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-atan2.md -->

#### `degToRad()`

Converts an angle from degrees to radians.

<SignatureBlock label="Signature">degToRad(degrees: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-degToRad.md -->

#### `radToDeg()`

Converts an angle from radians to degrees.

<SignatureBlock label="Signature">radToDeg(radians: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/trigonometry-radToDeg.md -->
