---
title: tyrus:net types
description: Handle, packet, address, and error types exported by tyrus:net.
---

# tyrus:net types

`tyrus:net` exports structured address and packet values, structured recoverable errors, and explicit TCP and UDP handles.

::: tip Types must be imported by name
`IpAddress`, `SocketAddress`, `UdpPacket`, `UdpSocket`, `TcpStream`, `TcpListener`, and `NetError` must be imported explicitly whenever their fields or methods are used.
:::

## IpAddress

`IpAddress` is returned by [`parseIpAddress()`](./parse-ip-address.md). It represents one validated IPv4 or IPv6 address.

| Field | Type | Description |
|-------|------|-------------|
| `address` | `string` | Canonical textual address, such as `"192.0.2.10"` or `"2001:db8::10"`. |
| `family` | `string` | Address family: `"IPv4"` or `"IPv6"`. |

<!-- @include: ../../.fragments/modules/net/types/ipaddress.md -->

## SocketAddress

`SocketAddress` is returned by [`parseSocketAddress()`](./parse-socket-address.md). It combines a validated IP address with its port.

| Field | Type | Description |
|-------|------|-------------|
| `ip` | `IpAddress` | The parsed IPv4 or IPv6 address. |
| `port` | `u16` | Port number from `0` through `65535`. |

<!-- @include: ../../.fragments/modules/net/types/socketaddress.md -->

## UdpPacket

`UdpPacket` is returned by [`UdpSocket.receiveFrom()`](#udpsocket-methods). It contains one received datagram and its sender address.

| Field | Type | Description |
|-------|------|-------------|
| `data` | `u8[]` | Datagram payload, truncated to the `maxBytes` requested by `receiveFrom`. |
| `address` | `string` | Sender socket address, such as `"127.0.0.1:50000"`. |

<!-- @include: ../../.fragments/modules/net/types/udppacket.md -->

## NetError

`NetError` is the recoverable error type for asynchronous network operations.

`NetError` is an `Error`: a `catch (error)` without an annotation receives it, and `error is NetError` narrows the binding to it, with every field of `NetError`.

| Field | Type | Description |
|-------|------|-------------|
| `code` | `string` | Stable machine-readable code, such as `"ERR_NET_TIMEOUT"`. |
| `message` | `string` | Human-readable explanation of the failure. |

<!-- @include: ../../.fragments/modules/net/types/neterror.md -->

## UdpSocket methods

`UdpSocket` is returned by [`udpBind()`](./udp-bind.md). Close it after its final operation; later calls return `Err(NetError)` with `ERR_NET_CLOSED`.

| Method | Signature | Description |
|--------|-----------|-------------|
| `sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Sends one datagram and returns the number of bytes sent. |
| `receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Receives one datagram and its sender address. |
| `setBroadcast` | `(enabled: boolean) => Promise<void>` | Enables or disables broadcast transmission. |
| `setTtl` | `(ttl: u32) => Promise<void>` | Sets IP TTL; valid values are `1` through `255`. |
| `localAddress` | `() => string` | Returns the bound local address. |
| `close` | `() => void` | Closes the socket and interrupts pending receives. |

<!-- @include: ../../.fragments/modules/net/types/udpsocket-methods.md -->

## TcpStream methods

`TcpStream` is returned by [`tcpConnect()`](./tcp-connect.md) and [`TcpListener.accept()`](#tcplistener-methods). Close it after its final operation.

| Method | Signature | Description |
|--------|-----------|-------------|
| `read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Reads up to `maxBytes`; EOF is an empty array. |
| `write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Writes bytes and returns the number written. |
| `shutdown` | `() => Promise<Result<void, NetError>>` | Closes the write side while retaining the read side. |
| `localAddress` | `() => string` | Returns the local socket address. |
| `peerAddress` | `() => string` | Returns the remote socket address. |
| `stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Hands the connection to a bounded byte duplex; closing its writable side shuts down only the write direction. Afterwards `read`, `write` and `shutdown` return `Err(NetError)` with `ERR_NET_STREAMED`. |
| `close` | `() => void` | Closes the stream and interrupts pending I/O. |

<!-- @include: ../../.fragments/modules/net/types/tcpstream-methods.md -->

## TcpListener methods

`TcpListener` is returned by [`tcpListen()`](./tcp-listen.md). Close it after it stops accepting connections.

| Method | Signature | Description |
|--------|-----------|-------------|
| `accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Accepts the next connection. |
| `localAddress` | `() => string` | Returns the bound listening address. |
| `close` | `() => void` | Closes the listener and interrupts pending accepts. |

<!-- @include: ../../.fragments/modules/net/types/tcplistener-methods.md -->

## Error handling

::: danger error
<!-- @include: ../../.fragments/modules/net/types/error-handling.md -->
:::
