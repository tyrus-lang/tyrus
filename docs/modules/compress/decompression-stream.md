---
title: decompressionStream
description: Decompress a byte stream safely, with constant memory.
---

# decompressionStream

`decompressionStream` returns a `TransformStream<u8[], u8[]>` that restores Gzip, zlib, raw Deflate, or Brotli data as it arrives. Without `format`, it identifies Gzip and Deflate from the first bytes; Brotli and raw Deflate need `format`.

The output leaves in pieces of at most 64 KiB, each one waiting for room in the output before the next is produced, so memory stays constant even when a small input expands enormously. The limits apply to the whole stream: past `maxOutputBytes` or `maxRatio`, the stream fails instead of producing more.

<SignatureBlock label="Signature">decompressionStream(options?: DecompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/compress/decompression-stream/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/compress/decompression-stream/maximal.md -->

## Errors

A failure reaches both sides as a [`CompressError`](./types.md#compresserror), with the same [codes](./index.md#error-codes) as `decompress`: data that ends early or carries bytes after its end is `ERR_COMPRESS_INVALID_DATA`, and the limits are `ERR_COMPRESS_OUTPUT_LIMIT` and `ERR_COMPRESS_RATIO_LIMIT`. The default `maxOutputBytes` of a stream is 4 GiB, since a stream never holds its output.
