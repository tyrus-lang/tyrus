---
title: udpBind
description: Vincula un socket UDP asíncrono.
---

# udpBind

`udpBind` vincula un socket UDP local y devuelve un handle `UdpSocket` explícito.

El socket devuelto puede enviar y recibir datagramas de forma asíncrona. El handle debe cerrarse cuando ya no sea necesario.

<SignatureBlock label="Firma">udpBind(address: string): Promise&lt;Result&lt;UdpSocket, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/udp-bind/udpbind.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/udp-bind/error-handling.md -->
:::

Los fallos de bind devuelven `Err(NetError)` con `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION` u otro código `ERR_NET_*` dedicado.
