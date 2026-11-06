---
title: TransformStream
description: Pair differently typed input and output stream capabilities.
---

# TransformStream

`TransformStream<TInput, TOutput>` exposes a writable input and readable output with independent payload types. It carries no implicit codec or transform function. The native transforms that do have one, such as [`compressionStream`](../compress/compression-stream.md) and [`decompressionStream`](../compress/decompression-stream.md), return a `TransformStream<u8[], u8[]>` whose output is the transformed input.

<SignatureBlock label="Signature">TransformStream&lt;TInput, TOutput&gt;</SignatureBlock>

## Members

| Member | Signature | Purpose |
| --- | --- | --- |
| `writable` | `WritableStream<TInput>` | Sends input values. |
| `readable` | `ReadableStream<TOutput>` | Receives output values. |
| `close` | `() => Promise<void>` | Closes both sides together. |

## Usage

<!-- @include: ../../.fragments/modules/stream/transform-stream.md -->

## Errors

`close()` rejects with `StreamError` if either side fails. The type parameters prevent writing output values to the input side or reading input values from the output side.
