---
title: Exponentials and logarithms
description: Exponential and logarithmic functions exported by tyrus:math.
---

# Exponentials and logarithms

| Function | Signature | Description |
|----------|-----------|-------------|
| [`exp`](#exp) | `(x: f64) => f64` | `e^x` |
| [`log`](#log) | `(x: f64) => f64` | Natural logarithm |
| [`log10`](#log10) | `(x: f64) => f64` | Base-10 logarithm |
| [`log2`](#log2) | `(x: f64) => f64` | Base-2 logarithm |
| [`pow`](#pow) | `(base: f64, exp: f64) => f64` | Power |

#### `exp()`

Returns Euler's number `e` raised to the power of `x`.

<SignatureBlock label="Signature">exp(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/exponentials-exp.md -->

#### `log()`

Returns the natural logarithm (base `e`) of `x`.

<SignatureBlock label="Signature">log(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/exponentials-log.md -->

#### `log10()`

Returns the base-10 logarithm of `x`.

<SignatureBlock label="Signature">log10(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/exponentials-log10.md -->

#### `log2()`

Returns the base-2 logarithm of `x`.

<SignatureBlock label="Signature">log2(x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/exponentials-log2.md -->

#### `pow()`

Returns `base` raised to the power of `exp`.

<SignatureBlock label="Signature">pow(base: f64, exp: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/exponentials-pow.md -->
