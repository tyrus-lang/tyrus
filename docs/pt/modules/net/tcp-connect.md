---
title: tcpConnect
description: Abre uma conexão TCP assíncrona.
---

# tcpConnect

`tcpConnect` conecta a um endpoint TCP e retorna um handle `TcpStream` dentro de `Ok`; falhas recuperáveis de conexão retornam `Err(NetError)`.

O timeout opcional é expresso em milissegundos. O stream retornado oferece leitura, escrita, shutdown e close assíncronos.

<SignatureBlock label="Assinatura">tcpConnect(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpStream, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/tcp-connect/tcpconnect.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/tcp-connect/error-handling.md -->
:::

Falhas de conexão cumprem com `Err(NetError)` usando `ERR_NET_TIMEOUT`, `ERR_NET_CONNECTION_REFUSED` ou outro código `ERR_NET_*` dedicado.
