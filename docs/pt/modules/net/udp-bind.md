---
title: udpBind
description: Vincula um socket UDP assíncrono.
---

# udpBind

`udpBind` vincula um socket UDP local e retorna um handle `UdpSocket` explícito.

O socket retornado pode enviar e receber datagramas de forma assíncrona. O handle deve ser fechado quando não for mais necessário.

<SignatureBlock label="Assinatura">udpBind(address: string): Promise&lt;Result&lt;UdpSocket, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/udp-bind/udpbind.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/udp-bind/error-handling.md -->
:::

Falhas de bind retornam `Err(NetError)` com `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION` ou outro código `ERR_NET_*` dedicado.
