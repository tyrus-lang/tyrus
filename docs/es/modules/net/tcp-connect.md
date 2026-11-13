---
title: tcpConnect
description: Abre una conexión TCP asíncrona.
---

# tcpConnect

`tcpConnect` conecta con un endpoint TCP y devuelve un handle `TcpStream` dentro de `Ok`; los fallos recuperables de conexión devuelven `Err(NetError)`.

El timeout opcional se expresa en milisegundos. El stream devuelto ofrece lectura, escritura, shutdown y close asíncronos.

<SignatureBlock label="Firma">tcpConnect(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpStream, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/tcp-connect/tcpconnect.md -->

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/net/tcp-connect/error-handling.md -->
:::

Los fallos de conexión cumplen con `Err(NetError)` usando `ERR_NET_TIMEOUT`, `ERR_NET_CONNECTION_REFUSED` u otro código `ERR_NET_*` dedicado.
