---
title: simdAdd
description: Suma vectorizada elemento a elemento de arrays numéricos del mismo tamaño.
---

# simdAdd

`simdAdd` suma los elementos correspondientes de dos arrays del mismo tamaño y devuelve un array nuevo dentro de `Result<T[], SimdError>`.

<SignatureBlock label="Firma">simdAdd&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/add.md -->

Si ambos argumentos son arrays fijos con longitudes diferentes, el compilador informa `E0308`. Si los tamaños solo se conocen en runtime, `simdAdd` devuelve `Err` con el código `ERR_SIMD_LENGTH_MISMATCH`. Si no se puede asignar el array de resultado, devuelve `ERR_SIMD_OUT_OF_MEMORY`. Estos errores recuperables no provocan panic. Todas las lanes enteras usan wrap por defecto. Por ejemplo, `120 + 120` como `i8` da `-16`, y `255 + 1` como `u8` da `0`, no `256`. Los literales enteros deben caber en el tipo declarado para los elementos. Los tipos aceptados son `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` y `f64`; `i128`, `u128`, `f128`, `isize`, `usize` y `decimal` no están aceptados. El compilador elige los anchos vectoriales y maneja con seguridad los elementos restantes; la API no promete una aceleración fija entre objetivos.

Los arrays de tamaño fijo también funcionan como argumentos directos. Su longitud se conoce en compilación y la operación SIMD lee sus elementos sin asignar antes un array dinámico de entrada.

## Errores

::: danger error
<!-- @include: ../../../.fragments/modules/simd/add-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/add-type-mismatch.md -->
:::
