---
title: Streams
description: Move bounded sequences of values between native Tyrus APIs with backpressure.
---

# Streams

`tyrus:stream` defines capability-separated streams for moving values progressively. A readable capability cannot write, a writable capability cannot read, and duplex and transform streams expose their two sides explicitly.

::: warning Integration status
The shared stream layer is complete: its types, methods, native ABI, bounded backpressure queue, lifecycle, and editor support are available. Filesystem adapters are public through `readFileStream` and `writeFileStream`, process adapters are public through `ChildProcess`, and HTTP exposes progressive request and response bodies through `StreamBody`, `fetch.stream(...)`, and `FetchOptions.bodyStream`. TCP and WebSocket return bounded byte duplexes through `TcpStream.stream()` and `WebSocket.stream()`; each WebSocket chunk is one complete binary message.
:::

## Imports

<!-- @include: ../../.fragments/modules/stream/api.md -->

## Exported items

| Item | Signature | Purpose |
| --- | --- | --- |
| [`ReadableStream`](./readable-stream.md) | `<T>` | Receives values progressively. |
| [`WritableStream`](./writable-stream.md) | `<T>` | Sends values progressively. |
| [`DuplexStream`](./duplex-stream.md) | `<TRead, TWrite>` | Combines independent readable and writable sides. |
| [`TransformStream`](./transform-stream.md) | `<TInput, TOutput>` | Connects an input side to a separately typed output side. |
| [`StreamError`](./errors.md) | `code: string, message: string` | Describes a recoverable stream failure. |

## Backpressure

Every stream buffer has a finite capacity. When the destination has no room, `write()` remains pending until the consumer reads enough data. This prevents memory usage from growing with the total size of a file or transfer.

For binary streams, chunks use `u8[]` and capacity is measured in bytes. A chunk larger than the complete buffer capacity is rejected immediately instead of waiting for space that can never become available.

## Reading and closing

`read()` resolves with the next chunk. After the writer closes, already accepted chunks remain readable; only after the queue is drained does `read()` resolve with an empty `T?`. Closing a readable side abandons unread chunks and wakes blocked writers with an error.

Closing is idempotent. `pipeTo()` reads from its source, waits for each destination write, closes the destination after normal end-of-stream, and propagates failures without replacing their original structured error.

## Ownership and concurrency

An accepted chunk belongs to the stream until `read()` transfers it to the consumer. A write cancelled before acceptance keeps the queue unchanged and releases its pending payload. In the initial contract, only one mutable operation may be pending on each stream side; invalid concurrent operations reject with `StreamError`.

## Errors

Recoverable stream failures use `StreamError`. Its stable `code` is suitable for programmatic comparison and its `message` is intended for people. Errors originating in another native module preserve that module's structured error when crossing `pipeTo()`.

## Binary transport

Encoded video, encoded audio, archives, documents, and arbitrary files all travel as opaque `u8[]` chunks. Streams preserve bytes and apply no codec, framing, or media interpretation.
