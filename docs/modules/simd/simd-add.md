---
title: simdAdd
description: Element-wise vectorized addition for equal-length numeric arrays.
---

# simdAdd

`simdAdd` adds corresponding elements from two equal-length arrays and returns a new array inside `Result<T[], SimdError>`.

<SignatureBlock label="Signature">simdAdd&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/simd/add.md -->

If both inputs are fixed-size arrays with different lengths, the compiler reports `E0308`. If the lengths are only known at runtime, `simdAdd` returns `Err` with code `ERR_SIMD_LENGTH_MISMATCH`. If the result array cannot be allocated, it returns `ERR_SIMD_OUT_OF_MEMORY`. These recoverable failures do not panic. All integer lanes use wrapping arithmetic by default. For example, `120 + 120` as `i8` returns `-16`, and `255 + 1` as `u8` returns `0`, not `256`. Integer literals must still fit their declared element type at compile time. Supported element types are `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32`, and `f64`; `i128`, `u128`, `f128`, `isize`, `usize`, and `decimal` are not accepted. The compiler selects vector widths and handles remaining elements safely; the API does not promise a fixed speedup across targets.

Fixed-size arrays work as direct arguments too. Their length is known at compile time, and the SIMD operation reads their elements without first allocating a dynamic input array.

## Errors

::: danger error
<!-- @include: ../../.fragments/modules/simd/add-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../.fragments/modules/simd/add-type-mismatch.md -->
:::
