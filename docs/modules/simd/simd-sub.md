---
title: simdSub
description: Element-wise vectorized subtraction for equal-length numeric arrays.
---

# simdSub

`simdSub` subtracts corresponding elements in the right array from the left array and returns a new array inside `Result<T[], SimdError>`.

<SignatureBlock label="Signature">simdSub&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/simd/sub.md -->

The supported element types are `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, and `f64`. Signed and unsigned integer subtraction wraps by default at the element width; for example, subtracting `1` from `0` as `u8` produces `255`. Integer literals must still fit their declared element type at compile time. Dynamic input length mismatch and result allocation failure return `SimdError` values. Different fixed input lengths are rejected with `E0308` at compile time.

## Errors

::: danger error
<!-- @include: ../../.fragments/modules/simd/sub-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../.fragments/modules/simd/sub-type-mismatch.md -->
:::
