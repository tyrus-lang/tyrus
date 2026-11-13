---
title: reverseLookup
description: Resuelve una dirección textual de vuelta a un nombre de host.
---

# reverseLookup

`reverseLookup` realiza una resolución inversa de nombre para una dirección.

La búsqueda es asíncrona y acepta un timeout opcional en milisegundos. Los fallos de red cumplen la promesa con `Err(NetError)`.

<SignatureBlock label="Firma">reverseLookup(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/reverse-lookup/reverselookup.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/reverse-lookup/error-handling.md -->
:::

Los fallos habituales incluyen `ERR_NET_INVALID_ADDRESS`, `ERR_NET_RESOLVE` y `ERR_NET_TIMEOUT`.
