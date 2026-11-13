---
title: traceroute
description: Traza saltos IPv4 con sondas UDP y respuestas ICMP.
---

# traceroute

`traceroute` informa los saltos IPv4 que responden dentro de `Ok`; Linux y macOS usan sondas UDP con TTL creciente y reciben respuestas ICMP, mientras que Windows usa sondas ICMP Echo con TTL creciente. Los fallos recuperables de la sonda se devuelven como `Err(NetError)`.

El resultado contiene como máximo 30 saltos. Un salto que no responde antes del timeout se representa con `"*"`. La función está disponible cuando `supportsIcmp()` devuelve true. macOS requiere permiso para abrir un socket ICMP raw.

<SignatureBlock label="Firma">traceroute(address: string, timeoutMs?: u64): Promise&lt;Result&lt;string[], NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/traceroute/traceroute.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/traceroute/error-handling.md -->
:::

Los fallos usan `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, `ERR_NET_TIMEOUT` o `ERR_NET_UNSUPPORTED`.
