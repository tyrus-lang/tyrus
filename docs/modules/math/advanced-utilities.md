---
title: Advanced utilities
description: Advanced utility math functions exported by tyrus:math.
---

# Advanced utilities

| Function | Signature | Description |
|----------|-----------|-------------|
| [`approxEqual`](#approxequal) | `(a: f64, b: f64, tolerance?: f64) => boolean` | Compare floats within tolerance |
| [`lerp`](#lerp) | `(start: f64, end: f64, amount: f64) => f64` | Linear interpolation |
| [`inverseLerp`](#inverselerp) | `(start: f64, end: f64, value: f64) => f64` | Inverse linear interpolation |
| [`remap`](#remap) | `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remap value between ranges |
| [`smoothstep`](#smoothstep) | `(edge0: f64, edge1: f64, x: f64) => f64` | Smooth Hermite interpolation |
| [`gcd`](#gcd) | `(a: u32, b: u32) => u32` | Greatest common divisor |
| [`lcm`](#lcm) | `(a: u32, b: u32) => u32` | Least common multiple |
| [`randomRange`](#randomrange) | `(min: f64, max: f64) => f64` | Random float in half-open interval |

#### `approxEqual()`

Checks if two floats are equal within an optional tolerance.

<SignatureBlock label="Signature">approxEqual(a: f64, b: f64, tolerance?: f64): boolean</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-approxEqual.md -->

#### `lerp()`

Linearly interpolates between two values.

<SignatureBlock label="Signature">lerp(start: f64, end: f64, amount: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-lerp.md -->

#### `inverseLerp()`

Returns how far a value is between two endpoints as a normalized fraction.

<SignatureBlock label="Signature">inverseLerp(start: f64, end: f64, value: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-inverseLerp.md -->

#### `remap()`

Remaps a value from one range to another.

<SignatureBlock label="Signature">remap(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-remap.md -->

#### `smoothstep()`

Returns smooth Hermite interpolation between two edges.

<SignatureBlock label="Signature">smoothstep(edge0: f64, edge1: f64, x: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-smoothstep.md -->

#### `gcd()`

Returns the greatest common divisor of two unsigned integers.

<SignatureBlock label="Signature">gcd(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-gcd.md -->

#### `lcm()`

Returns the least common multiple of two unsigned integers.

<SignatureBlock label="Signature">lcm(a: u32, b: u32): u32</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-lcm.md -->

#### `randomRange()`

Returns a random floating-point number in the half-open interval `[min, max)`.

<SignatureBlock label="Signature">randomRange(min: f64, max: f64): f64</SignatureBlock>

<!-- @include: ../../.fragments/modules/math/advanced-randomRange.md -->
