---
title: simdSub
description: Resta vectorizada elemento a elemento de arrays numéricos del mismo tamaño.
---

# simdSub

`simdSub` resta cada elemento del array derecho del elemento correspondiente del array izquierdo y devuelve un array nuevo dentro de `Result<T[], SimdError>`.

<SignatureBlock label="Firma">simdSub&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/sub.md -->

Los tipos aceptados son `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `f32` y `f64`. La resta de enteros con y sin signo usa wrap por defecto según el ancho del elemento; por ejemplo, restar `1` a `0` como `u8` da `255`. Los literales enteros deben caber en el tipo declarado para los elementos. Las diferencias de tamaño dinámico y los fallos al asignar el resultado devuelven `SimdError`. Los arrays fijos con longitudes diferentes se rechazan con `E0308` en compilación.

## Errores

::: danger error
<!-- @include: ../../../.fragments/modules/simd/sub-length-mismatch.md -->
:::

::: danger error
<!-- @include: ../../../.fragments/modules/simd/sub-type-mismatch.md -->
:::
