---
title: simdDiv
description: Vectorized element-wise division for numeric arrays.
---

# simdDiv

`simdDiv` divides corresponding elements and returns a new array in `Result<T[], SimdError>`.

<SignatureBlock label="Signature">simdDiv&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/simd/div.md -->

Integer division truncates toward zero. A zero divisor in any integer lane returns `ERR_SIMD_DIVISION_BY_ZERO`. Signed minimum divided by `-1` returns `ERR_SIMD_INTEGER_OVERFLOW` because the exact quotient is outside the lane type. The operation checks every integer lane before division, so an invalid lane never reaches an integer divide. Floating-point division follows IEEE behavior and returns infinities or NaN in `Ok` when appropriate. Dynamic length mismatch and result allocation failure also return `SimdError`. Fixed-size arrays can be passed directly; unequal fixed lengths are rejected with `E0308` at compile time. Supported element types are `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, and `f64`.

## Floating-point division by zero

Floating-point division by zero follows IEEE 754 and returns these values in `Ok`: `1.0 / 0.0` produces `Infinity`, `-1.0 / 0.0` produces `-Infinity`, and `0.0 / 0.0` produces `NaN`.

<!-- @include: ../../.fragments/modules/simd/div-float.md -->

## Errors

::: danger error
A zero integer divisor returns a recoverable error:
<!-- @include: ../../.fragments/modules/simd/div-zero-error.md -->
:::

::: danger error
Signed minimum divided by `-1` also returns a recoverable error:
<!-- @include: ../../.fragments/modules/simd/div-overflow-error.md -->
:::
