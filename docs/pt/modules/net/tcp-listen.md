---
title: tcpListen
description: Vincula um listener TCP assíncrono.
---

# tcpListen

`tcpListen` vincula um endpoint TCP e retorna um handle `TcpListener` dentro de `Ok`; falhas recuperáveis de bind retornam `Err(NetError)`.

Use `accept` para receber conexões. O timeout opcional se aplica ao bind e é expresso em milissegundos.

<SignatureBlock label="Assinatura">tcpListen(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpListener, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/net/tcp-listen/tcplisten.md -->

## Tratamento de erro

::: danger error
<!-- @include: ../../../.fragments/modules/net/tcp-listen/error-handling.md -->
:::

Falhas de bind cumprem com `Err(NetError)` usando `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION` ou outro código `ERR_NET_*` dedicado.
