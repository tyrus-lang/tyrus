---
title: DuplexStream
description: Pair independent readable and writable stream capabilities.
---

# DuplexStream

`DuplexStream<TRead, TWrite>` groups one readable side and one writable side without granting either capability extra permissions. A TCP connection becomes one through `TcpStream.stream()`, and a binary WebSocket connection through `WebSocket.stream()`, both as `DuplexStream<u8[], u8[]>`. TCP chunks are transport bytes; every WebSocket chunk is one complete binary message.

<SignatureBlock label="Signature">DuplexStream&lt;TRead, TWrite&gt;</SignatureBlock>

## Members

| Member | Signature | Purpose |
| --- | --- | --- |
| `readable` | `ReadableStream<TRead>` | Receives values. |
| `writable` | `WritableStream<TWrite>` | Sends values. |
| `close` | `() => Promise<void>` | Closes both sides together. |

## Usage

<!-- @include: ../../.fragments/modules/stream/duplex-stream.md -->

## Errors

`close()` rejects with `StreamError` if either side cannot complete its lifecycle. Read and write failures remain isolated to their corresponding capability.
