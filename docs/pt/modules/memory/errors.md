---
title: MemoryError
description: Falhas recuperáveis de apagamento seguro.
---

# MemoryError

<SignatureBlock label="Assinatura">interface MemoryError { readonly code: string; readonly message: string; }</SignatureBlock>

`MemoryError` é retornado na variante `Err` de `Result<void, MemoryError>` e pode ser inspecionado por `match` ou propagado com `?`.

`MemoryError` possui os campos estáveis `code` e `message` legível. `zeroize` pode retornar `ERR_MEMORY_NOT_OWNED`, `ERR_MEMORY_SHARED_VALUE` ou `ERR_MEMORY_INVALID_VALUE`.

<!-- @include: ../../../.fragments/modules/memory/errors/memoryerror.md -->
