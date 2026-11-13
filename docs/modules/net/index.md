---
title: tyrus:net
description: Native address, name resolution, asynchronous TCP and UDP APIs, and capability-gated network probes.
---

# tyrus:net

`tyrus:net` provides validated address parsing, system name resolution, asynchronous TCP and UDP networking, and capability-gated network probes for native programs.

## Imports

<SignatureBlock label="Imports">import { NetError, TcpListener, TcpStream, UdpPacket, UdpSocket, tcpConnect, tcpListen, udpBind, ping, traceroute, supportsIcmp, parseIpAddress, parseSocketAddress, resolve, reverseLookup } from "tyrus:net";</SignatureBlock>

## Address parsing and resolution

| Item | Signature | Purpose |
|------|-----------|---------|
| [`parseIpAddress`](./parse-ip-address.md) | `(value: string) => IpAddress?` | Parses an IPv4 or IPv6 address without performing I/O. |
| [`parseSocketAddress`](./parse-socket-address.md) | `(value: string) => SocketAddress?` | Parses an address with a port, including bracketed IPv6 notation. |
| [`resolve`](./resolve.md) | `(host: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Resolves a host through the system resolver. |
| [`reverseLookup`](./reverse-lookup.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string, NetError>>` | Performs reverse name lookup for an address. |

Invalid address text returns an absent optional from the parsing functions. `resolve` and `reverseLookup` fulfill with `Err(NetError)` when resolution fails, and `ping` and `traceroute` do the same for recoverable probe failures; the stable `code` identifies the failure and the `message` is human-readable.

## Contract and support

Network handles are explicit imported types. Methods borrow their handle, so a socket remains usable until `close()`; closing a TCP handle interrupts pending I/O and later operations return `Err(NetError)` with `ERR_NET_CLOSED`, and UDP failures also return `Err(NetError)`. TCP and UDP operations may run concurrently, with each socket operation serialized by the handle. `NetworkInterface` remains provided by `tyrus:os` to avoid a duplicate network-interface type.

| Code | Meaning |
|------|---------|
| `ERR_NET_INVALID_ADDRESS` | The address text is invalid. |
| `ERR_NET_TIMEOUT` | The operation exceeded its timeout. |
| `ERR_NET_PERMISSION` | The operating system or sandbox denied the operation. |
| `ERR_NET_CLOSED` | The socket was closed. |
| `ERR_NET_UNSUPPORTED` | The current platform has no implementation for the operation. |

The initial support matrix is Linux and Windows for address parsing, resolution, TCP, UDP, ping, and traceroute; macOS supports address parsing, resolution, TCP, and UDP, while ICMP probes fulfill with `Err(NetError)` using `ERR_NET_UNSUPPORTED`.

## TCP

`tcpConnect(address, timeoutMs?)` returns `Promise<Result<TcpStream, NetError>>`. `tcpListen(address, timeoutMs?)` returns `Promise<Result<TcpListener, NetError>>`. The optional timeout is expressed in milliseconds.

| Item | Signature | Purpose |
|------|-----------|---------|
| `TcpStream.read` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<u8[], NetError>>` | Reads up to the requested number of bytes; EOF is an empty array. |
| `TcpStream.write` | `(data: u8[], timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Writes bytes and returns the number written, which may be smaller than the input length. |
| `TcpStream.shutdown` | `() => Promise<Result<void, NetError>>` | Shuts down the stream's write side. |
| `TcpStream.localAddress` | `() => string` | Returns the local socket address. |
| `TcpStream.peerAddress` | `() => string` | Returns the remote socket address. |
| `TcpStream.stream` | `() => Promise<Result<DuplexStream<u8[], u8[]>, NetError>>` | Hands the connection to a bounded byte duplex. |
| `TcpStream.close` | `() => void` | Closes the stream and wakes pending I/O. |
| `TcpListener.accept` | `(timeoutMs?: u64) => Promise<Result<TcpStream, NetError>>` | Accepts the next connection. |
| `TcpListener.localAddress` | `() => string` | Returns the listening socket address. |
| `TcpListener.close` | `() => void` | Closes the listener and wakes a pending accept. |

Closing a stream or listener makes later handle operations return `Err(NetError)` with `ERR_NET_CLOSED` and interrupts pending reads or accepts without blocking the executor. TCP constructors and handle methods fulfill with `Result<T, NetError>` for recoverable failures; timeouts and other failures carry dedicated `ERR_NET_*` codes. `stream()` hands the connection to a `DuplexStream<u8[], u8[]>` whose sides apply backpressure; closing its writable side shuts down only the write direction, so the peer reads end-of-file while this side can still read the answer. From then on the duplex owns the socket, and the stream's own `read`, `write` and `shutdown` return `Err(NetError)` with `ERR_NET_STREAMED`.

## UDP

`udpBind(address)` returns `Promise<Result<UdpSocket, NetError>>`. `sendTo` sends one datagram and returns the number of bytes sent. `receiveFrom` returns the received bytes and the sender address; a datagram larger than `maxBytes` is truncated to that limit. The maximum datagram payload is 65,507 bytes. All recoverable UDP failures fulfill with `Err(NetError)`.

| Item | Signature | Purpose |
|------|-----------|---------|
| `UdpSocket.sendTo` | `(data: u8[], address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Sends one datagram to the destination address. |
| `UdpSocket.receiveFrom` | `(maxBytes: u32, timeoutMs?: u64) => Promise<Result<UdpPacket, NetError>>` | Receives one datagram, truncating it to `maxBytes`. |
| `UdpSocket.setBroadcast` | `(enabled: boolean) => Promise<Result<void, NetError>>` | Enables or disables broadcast transmission. |
| `UdpSocket.setTtl` | `(ttl: u32) => Promise<Result<void, NetError>>` | Sets the IP time-to-live value. |
| `UdpSocket.localAddress` | `() => string` | Returns the bound socket address. |
| `UdpSocket.close` | `() => void` | Closes the socket and wakes a pending receive. |
| `UdpPacket.data` | `u8[]` | Contains the datagram payload. |
| `UdpPacket.address` | `string` | Contains the sender socket address. |

Closing a UDP socket makes later operations return `Err(NetError)` with `ERR_NET_CLOSED`. A timeout returns `ERR_NET_TIMEOUT`; malformed input, bind failures, allocation failures, option failures, and receive failures use dedicated `ERR_NET_*` codes. TTL accepts values from 1 through 255 and returns `ERR_NET_INVALID_TTL` for other values. Broadcast and TTL changes return `ERR_NET_PERMISSION` when denied by the operating system and `ERR_NET_OPTION` for other platform errors.

## Network probes

`supportsIcmp()` reports whether the current platform can open its native ICMP probe handle or sockets. `ping` sends one ICMP Echo request. `traceroute` reports responding IPv4 hops, using "*" when a hop does not respond before the timeout; Linux sends UDP probes with increasing TTL values, while Windows sends ICMP Echo probes with increasing TTL values. macOS currently returns `false` and probes fulfill with `Err(NetError)` using `ERR_NET_UNSUPPORTED`; probes never replace ICMP with a TCP connection.

| Item | Signature | Purpose |
|------|-----------|---------|
| [`supportsIcmp`](./supports-icmp.md) | `() => boolean` | Reports whether ICMP probes are available. |
| [`ping`](./ping.md) | `(address: string, timeoutMs?: u64) => Promise<Result<u64, NetError>>` | Measures one ICMP probe and returns its round-trip time in milliseconds when supported. |
| [`traceroute`](./traceroute.md) | `(address: string, timeoutMs?: u64) => Promise<Result<string[], NetError>>` | Reports up to 30 IPv4 hops when supported. |

## Example

<!-- @include: ../../.fragments/modules/net/index/example.md -->
