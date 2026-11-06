---
title: simdMul
description: Element-wise vectorized multiplication for equal-length numeric arrays.
---

# simdMul

`simdMul` multiplies corresponding elements from two equal-length arrays and returns a new array inside `Result<T[], SimdError>`.

<SignatureBlock label="Signature">simdMul&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/simd/mul.md -->

The supported element types are `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, and `f64`. Signed and unsigned integer multiplication wraps by default at the element width; for example, multiplying `255` by `2` as `u8` produces `254`. Integer literals must still fit their declared element type at compile time. Dynamic input length mismatch and result allocation failure return `SimdError` values. Different fixed input lengths are rejected with `E0308` at compile time.

## Errors

::: danger error
<!-- @include: ../../.fragments/modules/simd/mul-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../.fragments/modules/simd/mul-type-mismatch.md -->
:::
