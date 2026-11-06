---
title: Functions
description: Mathematical functions exported by tyrus:math.
---

# Functions

All `tyrus:math` functions operate on `f64` unless noted otherwise and are emitted as direct native calls.

## [Basics and signs](./basics-and-signs.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`abs`](./basics-and-signs.md#abs) `(x: f64) => f64` | Absolute value |
| [`sign`](./basics-and-signs.md#sign) `(x: f64) => f64` | Sign (`1`, `-1`, or `0`) |
| [`sqrt`](./basics-and-signs.md#sqrt) `(x: f64) => f64` | Square root |
| [`cbrt`](./basics-and-signs.md#cbrt) `(x: f64) => f64` | Cube root |

## [Rounding](./rounding.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`floor`](./rounding.md#floor) `(x: f64) => f64` | Round down |
| [`ceil`](./rounding.md#ceil) `(x: f64) => f64` | Round up |
| [`trunc`](./rounding.md#trunc) `(x: f64) => f64` | Truncate toward zero |
| [`round`](./rounding.md#round) `(x: f64) => f64` | Round to nearest integer |
| [`roundTo`](./rounding.md#roundto) `(x: f64, decimals: i32) => f64` | Round to N decimal places |

## [Trigonometry](./trigonometry.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`sin`](./trigonometry.md#sin) `(x: f64) => f64` | Sine |
| [`cos`](./trigonometry.md#cos) `(x: f64) => f64` | Cosine |
| [`tan`](./trigonometry.md#tan) `(x: f64) => f64` | Tangent |
| [`asin`](./trigonometry.md#asin) `(x: f64) => f64` | Arc sine |
| [`acos`](./trigonometry.md#acos) `(x: f64) => f64` | Arc cosine |
| [`atan`](./trigonometry.md#atan) `(x: f64) => f64` | Arc tangent |
| [`atan2`](./trigonometry.md#atan2) `(y: f64, x: f64) => f64` | Arc tangent of `y/x` |
| [`degToRad`](./trigonometry.md#degtorad) `(degrees: f64) => f64` | Convert degrees to radians |
| [`radToDeg`](./trigonometry.md#radtodeg) `(radians: f64) => f64` | Convert radians to degrees |

## [Hyperbolic](./hyperbolic.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`sinh`](./hyperbolic.md#sinh) `(x: f64) => f64` | Hyperbolic sine |
| [`cosh`](./hyperbolic.md#cosh) `(x: f64) => f64` | Hyperbolic cosine |
| [`tanh`](./hyperbolic.md#tanh) `(x: f64) => f64` | Hyperbolic tangent |

## [Exponentials and logarithms](./exponentials-and-logarithms.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`exp`](./exponentials-and-logarithms.md#exp) `(x: f64) => f64` | `e^x` |
| [`log`](./exponentials-and-logarithms.md#log) `(x: f64) => f64` | Natural logarithm |
| [`log10`](./exponentials-and-logarithms.md#log10) `(x: f64) => f64` | Base-10 logarithm |
| [`log2`](./exponentials-and-logarithms.md#log2) `(x: f64) => f64` | Base-2 logarithm |
| [`pow`](./exponentials-and-logarithms.md#pow) `(base: f64, exp: f64) => f64` | Power |

## [Selection and low-level](./selection-and-low-level.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`max`](./selection-and-low-level.md#max) `(a: f64, b: f64) => f64` | Maximum |
| [`min`](./selection-and-low-level.md#min) `(a: f64, b: f64) => f64` | Minimum |
| [`clamp`](./selection-and-low-level.md#clamp) `(value: f64, min: f64, max: f64) => f64` | Clamp value to range |
| [`clampMin`](./selection-and-low-level.md#clampmin) `(value: f64, min: f64) => f64` | Enforce minimum |
| [`clampMax`](./selection-and-low-level.md#clampmax) `(value: f64, max: f64) => f64` | Enforce maximum |
| [`imul`](./selection-and-low-level.md#imul) `(a: i32, b: i32) => i32` | C-style 32-bit multiplication |
| [`clz32`](./selection-and-low-level.md#clz32) `(x: i32) => i32` | Count leading zeros in 32-bit integer |

## [Advanced utilities](./advanced-utilities.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`approxEqual`](./advanced-utilities.md#approxequal) `(a: f64, b: f64, tolerance?: f64) => boolean` | Compare floats within tolerance |
| [`lerp`](./advanced-utilities.md#lerp) `(start: f64, end: f64, amount: f64) => f64` | Linear interpolation |
| [`inverseLerp`](./advanced-utilities.md#inverselerp) `(start: f64, end: f64, value: f64) => f64` | Inverse linear interpolation |
| [`remap`](./advanced-utilities.md#remap) `(value: f64, inMin: f64, inMax: f64, outMin: f64, outMax: f64) => f64` | Remap value between ranges |
| [`smoothstep`](./advanced-utilities.md#smoothstep) `(edge0: f64, edge1: f64, x: f64) => f64` | Smooth Hermite interpolation |
| [`gcd`](./advanced-utilities.md#gcd) `(a: u32, b: u32) => u32` | Greatest common divisor |
| [`lcm`](./advanced-utilities.md#lcm) `(a: u32, b: u32) => u32` | Least common multiple |
| [`randomRange`](./advanced-utilities.md#randomrange) `(min: f64, max: f64) => f64` | Random float in half-open interval |

## [Statistics and combinatorics](./statistics-and-combinatorics.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`mean`](./statistics-and-combinatorics.md#mean) `(values: f64[]) => f64` | Arithmetic mean; `0.0` for empty array |
| [`median`](./statistics-and-combinatorics.md#median) `(values: f64[]) => f64` | Median; `0.0` for empty array |
| [`mode`](./statistics-and-combinatorics.md#mode) `(values: f64[]) => f64` | Most frequent value; `0.0` for empty array |
| [`variance`](./statistics-and-combinatorics.md#variance) `(values: f64[]) => f64` | Population variance; `0.0` for empty array |
| [`stdDev`](./statistics-and-combinatorics.md#stddev) `(values: f64[]) => f64` | Population standard deviation; `0.0` for empty array |
| [`percentile`](./statistics-and-combinatorics.md#percentile) `(p: f64, values: f64[]) => f64` | `p`-th percentile (0-100) with linear interpolation; `0.0` for empty array |
| [`factorial`](./statistics-and-combinatorics.md#factorial) `(n: u128) => u128` | `n!` |
| [`permutation`](./statistics-and-combinatorics.md#permutation) `(n: u128, k: u128) => u128` | `n! / (n - k)!` |
| [`combination`](./statistics-and-combinatorics.md#combination) `(n: u128, k: u128) => u128` | `n! / (k! * (n - k)!)` |

## [Number theory](./number-theory.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`isPrime`](./number-theory.md#isprime) `(n: u64) => boolean` | `true` if `n` is prime |
| [`nextPrime`](./number-theory.md#nextprime) `(n: u64) => u64` | Smallest prime strictly greater than `n` |
| [`primeFactors`](./number-theory.md#primefactors) `(n: u64) => u64[]` | Prime factors of `n` |
| [`fibonacci`](./number-theory.md#fibonacci) `(n: u64) => u64` | `n`-th Fibonacci number |

## [Geometry and numeric utilities](./geometry-and-numeric-utilities.md)

| Function | Signature | Description |
|----------|-----------|-------------|
| [`hypot`](./geometry-and-numeric-utilities.md#hypot) `(x: f64, y: f64) => f64` | Euclidean norm `sqrt(x^2 + y^2)` |
| [`toPolar`](./geometry-and-numeric-utilities.md#topolar) `(x: f64, y: f64) => [f64, f64]` | Convert Cartesian to polar `(r, theta)` |
| [`toCartesian`](./geometry-and-numeric-utilities.md#tocartesian) `(r: f64, theta: f64) => [f64, f64]` | Convert polar to Cartesian `(x, y)` |
| [`copysign`](./geometry-and-numeric-utilities.md#copysign) `(x: f64, y: f64) => f64` | Copy sign of `y` to `x` |
| [`fmod`](./geometry-and-numeric-utilities.md#fmod) `(x: f64, y: f64) => f64` | Floating-point remainder of `x / y` |
| [`logBase`](./geometry-and-numeric-utilities.md#logbase) `(x: f64, base: f64) => f64` | Logarithm of `x` with given `base` |
| [`powi`](./geometry-and-numeric-utilities.md#powi) `(x: f64, exp: i32) => f64` | `x` raised to integer power `exp` |
