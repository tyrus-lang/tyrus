---
title: compressionStream
description: Compress a byte stream of any size with constant memory.
---

# compressionStream

`compressionStream` returns a `TransformStream<u8[], u8[]>`: bytes written to its `writable` side leave its `readable` side compressed in the format you name. It never holds the whole input or output, so a file of any size is compressed with constant memory, and a slow reader holds the writer back instead of letting data pile up.

<SignatureBlock label="Signature">compressionStream(format: CompressFormat, options?: CompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/compress/compression-stream/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/compress/compression-stream/maximal.md -->

## Errors

A failure reaches both sides as a [`CompressError`](./types.md#compresserror): `read()` on the output and `write()` or `close()` on the input reject with it, and `catch (error: CompressError)` receives it. A `level` outside the range of the format makes the stream fail on its first use with `ERR_COMPRESS_INVALID_LEVEL`; a literal one is refused when the program is compiled (`E3312`).
