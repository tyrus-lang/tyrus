---
title: tcpListen
description: Vincula un listener TCP asíncrono.
---

# tcpListen

`tcpListen` vincula un endpoint TCP y devuelve un handle `TcpListener` dentro de `Ok`; los fallos recuperables de bind devuelven `Err(NetError)`.

Usa `accept` para recibir conexiones. El timeout opcional se aplica al bind y se expresa en milisegundos.

<SignatureBlock label="Firma">tcpListen(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpListener, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/tcp-listen/tcplisten.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/tcp-listen/error-handling.md -->
:::

Los fallos de bind cumplen con `Err(NetError)` usando `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION` u otro código `ERR_NET_*` dedicado.
