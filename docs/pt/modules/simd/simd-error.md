---
title: SimdError
description: Erros recuperáveis retornados pelas operações SIMD explícitas.
---

# SimdError

`SimdError` descreve falhas recuperáveis retornadas por funções de `tyrus:simd`.

<SignatureBlock label="Assinatura">interface SimdError { code: string; message: string; }</SignatureBlock>

`ERR_SIMD_LENGTH_MISMATCH` indica que os arrays de entrada têm tamanhos diferentes. `ERR_SIMD_OUT_OF_MEMORY` indica que não foi possível alocar o array de resultado. `ERR_SIMD_DIVISION_BY_ZERO` indica que uma lane inteira tem divisor zero. `ERR_SIMD_INTEGER_OVERFLOW` indica mínimo signed dividido por `-1`.
