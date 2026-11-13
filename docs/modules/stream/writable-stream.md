---
title: WritableStream
description: Send a bounded sequence of values progressively.
---

# WritableStream

`WritableStream<T>` is a write-only capability returned by native adapters such as file writes and child-process input.

<SignatureBlock label="Signature">WritableStream&lt;T&gt;</SignatureBlock>

## Methods

| Method | Signature | Purpose |
| --- | --- | --- |
| `write` | `(value: T) => Promise<void>` | Waits until the bounded queue accepts one value. |
| `close` | `() => Promise<void>` | Finishes accepted writes and closes the writable side. |

## Usage

<!-- @include: ../../.fragments/modules/stream/writable-stream.md -->

## Errors

`write()` rejects with `StreamError` for a closed, failed, or concurrently used stream, and immediately rejects a binary chunk larger than the buffer capacity. `close()` is idempotent.
