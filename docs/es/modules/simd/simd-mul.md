---
title: simdMul
description: Multiplicación vectorizada elemento a elemento de arrays numéricos del mismo tamaño.
---

# simdMul

`simdMul` multiplica los elementos correspondientes de dos arrays del mismo tamaño y devuelve un array nuevo dentro de `Result<T[], SimdError>`.

<SignatureBlock label="Firma">simdMul&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/mul.md -->

Los tipos aceptados son `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` y `f64`. La multiplicación de enteros con y sin signo usa wrap por defecto según el ancho del elemento; por ejemplo, multiplicar `255` por `2` como `u8` da `254`. Los literales enteros deben caber en el tipo declarado para los elementos. Las diferencias de tamaño dinámico y los fallos al asignar el resultado devuelven `SimdError`. Los arrays fijos con longitudes diferentes se rechazan con `E0308` en compilación.

## Errores

::: danger error
<!-- @include: ../../../.fragments/modules/simd/mul-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/mul-type-mismatch.md -->
:::
