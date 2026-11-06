---
title: resolve
description: Resuelve un nombre de host mediante el resolvedor del sistema.
---

# resolve

`resolve` resuelve un nombre de host o una dirección en direcciones textuales.

La resolución es asíncrona y acepta un timeout opcional en milisegundos. Los fallos de red cumplen la promesa con `Err(NetError)`.

<SignatureBlock label="Firma">resolve(host: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/resolve/resolve.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/resolve/error-handling.md -->
:::

Los fallos habituales incluyen `ERR_NET_RESOLVE` y `ERR_NET_TIMEOUT`.
