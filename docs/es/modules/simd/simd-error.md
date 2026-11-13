---
title: SimdError
description: Errores recuperables devueltos por las operaciones SIMD explícitas.
---

# SimdError

`SimdError` describe errores recuperables devueltos por las funciones de `tyrus:simd`.

<SignatureBlock label="Firma">interface SimdError { code: string; message: string; }</SignatureBlock>

`ERR_SIMD_LENGTH_MISMATCH` indica que los arrays de entrada tienen tamaños distintos. `ERR_SIMD_OUT_OF_MEMORY` indica que no se pudo asignar el array de resultado. `ERR_SIMD_DIVISION_BY_ZERO` indica que una lane entera tiene divisor cero. `ERR_SIMD_INTEGER_OVERFLOW` indica mínimo signed dividido por `-1`.
