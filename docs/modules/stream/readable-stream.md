---
title: ReadableStream
description: Receive a bounded sequence of values progressively.
---

# ReadableStream

`ReadableStream<T>` is a read-only capability returned by native adapters such as file reads and child-process output.

<SignatureBlock label="Signature">ReadableStream&lt;T&gt;</SignatureBlock>

## Methods

| Method | Signature | Purpose |
| --- | --- | --- |
| `read` | `() => Promise<T?>` | Resolves with the next value or empty at end-of-stream. |
| `pipeTo` | `(destination: WritableStream<T>) => Promise<void>` | Copies values progressively and closes the destination after normal EOF. |
| `close` | `() => Promise<void>` | Abandons unread values and wakes blocked writers. |

## Usage

<!-- @include: ../../.fragments/modules/stream/readable-stream.md -->

## Errors

`read()`, `pipeTo()`, and `close()` reject with `StreamError` when the stream has failed or an operation violates its lifecycle. `pipeTo()` preserves a structured error produced by its source or destination. Cancelling a pending `pipeTo()` operation fails both endpoints with `ERR_STREAM_CANCELLED` and discards their buffered chunks.
