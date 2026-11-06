---
title: simdDiv
description: División vectorizada elemento a elemento de arrays numéricos.
---

# simdDiv

`simdDiv` divide los elementos correspondientes y devuelve un array nuevo en `Result<T[], SimdError>`.

<SignatureBlock label="Firma">simdDiv&lt;T&gt;(left: T[], right: T[]): Result&lt;T[], SimdError&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/simd/div.md -->

La división entera trunca hacia cero. Si cualquier lane entera tiene divisor cero, el resultado es `ERR_SIMD_DIVISION_BY_ZERO`. El mínimo signed dividido por `-1` devuelve `ERR_SIMD_INTEGER_OVERFLOW`, porque el cociente exacto no cabe en el tipo de la lane. La operación verifica todas las lanes enteras antes de la división, así ninguna lane inválida llega a una división entera. La división de punto flotante sigue IEEE y devuelve infinito o NaN en `Ok` cuando corresponde. Las diferencias de tamaño dinámico y los fallos al asignar el resultado también devuelven `SimdError`. Los arrays de tamaño fijo se pueden pasar directamente; las longitudes fijas distintas se rechazan con `E0308` en compilación.

## División de punto flotante por cero

La división de punto flotante por cero sigue IEEE 754 y devuelve estos valores en `Ok`: `1.0 / 0.0` produce `Infinity`, `-1.0 / 0.0` produce `-Infinity` y `0.0 / 0.0` produce `NaN`.

<!-- @include: ../../../.fragments/modules/simd/div-float.md -->

## Errores

::: danger error
Un divisor entero igual a cero devuelve un error recuperable:
<!-- @include: ../../../.fragments/modules/simd/div-zero-error.md -->
:::

::: danger error
El mínimo signed dividido por `-1` también devuelve un error recuperable:
<!-- @include: ../../../.fragments/modules/simd/div-overflow-error.md -->
:::
