---
title: udpBind
description: Bind an asynchronous UDP socket.
---

# udpBind

`udpBind` binds a local UDP socket and returns an explicit `UdpSocket` handle.

The returned socket can send and receive datagrams asynchronously. The handle must be closed when it is no longer needed.

<SignatureBlock label="Signature">udpBind(address: string): Promise&lt;Result&lt;UdpSocket, NetError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/net/udp-bind/udpbind.md -->

## Error Handling

::: danger error
<!-- @include: ../../.fragments/modules/net/udp-bind/error-handling.md -->
:::

Binding failures return `Err(NetError)` with `ERR_NET_INVALID_ADDRESS`, `ERR_NET_PERMISSION`, or another dedicated `ERR_NET_*` code.
