---
title: ping
description: Mide un viaje de ida y vuelta ICMP Echo.
---

# ping

`ping` envía una solicitud ICMP Echo y devuelve el tiempo de ida y vuelta en milisegundos dentro de `Ok`; los fallos recuperables de la sonda se devuelven como `Err(NetError)`.

La función está disponible cuando `supportsIcmp()` devuelve true. Está implementada en Linux, macOS y Windows y devuelve `Err(NetError)` cuando la plataforma o el sandbox no ofrecen la capacidad necesaria. macOS requiere permiso para abrir un socket ICMP raw.

<SignatureBlock label="Firma">ping(address: string, timeoutMs?: u64): Promise&lt;Result&lt;u64, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/ping/ping.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/ping/error-handling.md -->
:::

Los fallos usan `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT` o `ERR_NET_UNSUPPORTED`.
