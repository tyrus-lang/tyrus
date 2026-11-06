---
title: Selection and low-level
description: Selection and low-level math functions exported by tyrus:math.
---

# Selection and low-level

| Function | Signature | Description |
|----------|-----------|-------------|
| [`max`](#max) `(a: f64, b: f64) => f64` | Maximum |
| [`min`](#min) `(a: f64, b: f64) => f64` | Minimum |
| [`clamp`](#clamp) `(value: f64, min: f64, max: f64) => f64` | Clamp value to range |
| [`clampMin`](#clampmin) `(value: f64, min: f64) => f64` | Enforce minimum |
| [`clampMax`](#clampmax) `(value: f64, max: f64) => f64` | Enforce maximum |
| [`imul`](#imul) `(a: i32, b: i32) => i32` | C-style 32-bit multiplication |
| [`clz32`](#clz32) `(x: i32) => i32` | Count leading zeros in 32-bit integer |

#### `max()`

Returns the larger of two numbers.

<SignatureBlock label="Signature">max(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-max.md -->

#### `min()`

Returns the smaller of two numbers.

<SignatureBlock label="Signature">min(a: f64, b: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-min.md -->

#### `clamp()`

Restricts a value to a given range.

<SignatureBlock label="Signature">clamp(value: f64, min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-clamp.md -->

#### `clampMin()`

Enforces a minimum value.

<SignatureBlock label="Signature">clampMin(value: f64, min: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-clampMin.md -->

#### `clampMax()`

Enforces a maximum value.

<SignatureBlock label="Signature">clampMax(value: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-clampMax.md -->

#### `imul()`

Performs C-style 32-bit integer multiplication, truncating overflow.

<SignatureBlock label="Signature">imul(a: i32, b: i32): i32</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-imul.md -->

#### `clz32()`

Counts the leading zero bits in a 32-bit integer.

<SignatureBlock label="Signature">clz32(x: i32): i32</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/selection-clz32.md -->
