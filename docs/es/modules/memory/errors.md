---
title: MemoryError
description: Fallos recuperables de borrado seguro.
---

# MemoryError

<SignatureBlock label="Firma">interface MemoryError { readonly code: string; readonly message: string; }</SignatureBlock>

`MemoryError` se devuelve en la variante `Err` de `Result<void, MemoryError>` y puede inspeccionarse con `match` o propagarse con `?`.

`MemoryError` tiene los campos estables `code` y `message` legible. `zeroize` puede devolver `ERR_MEMORY_NOT_OWNED`, `ERR_MEMORY_SHARED_VALUE` o `ERR_MEMORY_INVALID_VALUE`.

<!-- @include: ../../../.fragments/modules/memory/errors/memoryerror.md -->
