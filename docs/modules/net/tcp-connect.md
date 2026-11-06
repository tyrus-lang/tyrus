---
title: tcpConnect
description: Open an asynchronous TCP connection.
---

# tcpConnect

`tcpConnect` connects to a TCP endpoint and returns a `TcpStream` handle inside `Ok`; recoverable connection failures return `Err(NetError)`.

The optional timeout is expressed in milliseconds. The returned stream supports asynchronous reads, writes, shutdown, and close.

<SignatureBlock label="Signature">tcpConnect(address: string, timeoutMs?: u64): Promise&lt;Result&lt;TcpStream, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/tcp-connect/tcpconnect.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/tcp-connect/error-handling.md -->
:::

Connection failures fulfill with `Err(NetError)` using `ERR_NET_TIMEOUT`, `ERR_NET_CONNECTION_REFUSED`, or another dedicated `ERR_NET_*` code.
