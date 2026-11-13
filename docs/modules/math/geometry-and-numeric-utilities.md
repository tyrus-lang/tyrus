---
title: Geometry and numeric utilities
description: Geometry and numeric utility math functions exported by tyrus:math.
---

# Geometry and numeric utilities

| Function | Signature | Description |
|----------|-----------|-------------|
| [`hypot`](#hypot) | `(x: f64, y: f64) => f64` | Euclidean norm `sqrt(x^2 + y^2)` |
| [`toPolar`](#topolar) | `(x: f64, y: f64) => [f64, f64]` | Convert Cartesian to polar `(r, theta)` |
| [`toCartesian`](#tocartesian) | `(r: f64, theta: f64) => [f64, f64]` | Convert polar to Cartesian `(x, y)` |
| [`copysign`](#copysign) | `(x: f64, y: f64) => f64` | Copy sign of `y` to `x` |
| [`fmod`](#fmod) | `(x: f64, y: f64) => f64` | Floating-point remainder of `x / y` |
| [`logBase`](#logbase) | `(x: f64, base: f64) => f64` | Logarithm of `x` with given `base` |
| [`powi`](#powi) | `(x: f64, exp: i32) => f64` | `x` raised to integer power `exp` |

#### `hypot()`

Returns the Euclidean norm (hypotenuse) `sqrt(x^2 + y^2)` without intermediate overflow.

<SignatureBlock label="Signature">hypot(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-hypot.md -->

#### `toPolar()`

Converts Cartesian coordinates `(x, y)` to polar coordinates `(r, theta)`.

<SignatureBlock label="Signature">toPolar(x: f64, y: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-toPolar.md -->

#### `toCartesian()`

Converts polar coordinates `(r, theta)` to Cartesian coordinates `(x, y)`.

<SignatureBlock label="Signature">toCartesian(r: f64, theta: f64): [f64, f64]</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-toCartesian.md -->

#### `copysign()`

Returns a value with the magnitude of `x` and the sign of `y`.

<SignatureBlock label="Signature">copysign(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-copysign.md -->

#### `fmod()`

Returns the floating-point remainder of `x / y`.

<SignatureBlock label="Signature">fmod(x: f64, y: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-fmod.md -->

#### `logBase()`

Returns the logarithm of `x` with the given `base`.

<SignatureBlock label="Signature">logBase(x: f64, base: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-logBase.md -->

#### `powi()`

Returns `x` raised to an integer power `exp`.

<SignatureBlock label="Signature">powi(x: f64, exp: i32): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/geometry-powi.md -->
