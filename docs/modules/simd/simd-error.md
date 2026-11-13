---
title: SimdError
description: Recoverable errors returned by explicit SIMD operations.
---

# SimdError

`SimdError` describes recoverable failures returned by functions in `tyrus:simd`.

<SignatureBlock label="Signature">interface SimdError { code: string; message: string; }</SignatureBlock>

`ERR_SIMD_LENGTH_MISMATCH` indicates that the input arrays have different lengths. `ERR_SIMD_OUT_OF_MEMORY` indicates that the result array could not be allocated. `ERR_SIMD_DIVISION_BY_ZERO` indicates that an integer divisor lane is zero. `ERR_SIMD_INTEGER_OVERFLOW` indicates signed minimum divided by `-1`.
