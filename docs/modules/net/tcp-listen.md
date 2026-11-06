---
title: tcpListen
description: Bind an asynchronous TCP listener.
---

# tcpListen

`tcpListen` binds a TCP endpoint and returns a `TcpListener` handle inside `Ok`; recoverable bind failures return `Err(NetError)`.

Use `accept` to receive connections. The optional timeout applies to the bind operation and is expressed in milliseconds.

<SignatureBlock label="Signature">tcpListen(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpListener, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/tcp-listen/tcplisten.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/tcp-listen/error-handling.md -->
:::

Binding failures fulfill with `Err(NetError)` using `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, or another dedicated `ERR_NET_*` code.
