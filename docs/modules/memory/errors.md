---
title: MemoryError
description: Recoverable secure-erasure failures.
---

# MemoryError

<SignatureBlock label="Signature">interface MemoryError { readonly code: string; readonly message: string; }</SignatureBlock>

`zeroize` returns `MemoryError` in the `Err` variant of `Result<void, MemoryError>`. Inspect it with `match` or propagate it with `?`.

`MemoryError` has stable `code` and readable `message` fields. `zeroize` can return `ERR_MEMORY_NOT_OWNED`, `ERR_MEMORY_SHARED_VALUE`, or `ERR_MEMORY_INVALID_VALUE`.

<!-- @include: ../../.fragments/modules/memory/errors/memoryerror.md -->
