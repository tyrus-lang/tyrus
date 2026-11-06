---
title: decompress
description: Decompress bytes safely, detecting Gzip and Deflate from the data.
---

# decompress

`decompress` restores bytes produced by `compress` or by any other Gzip, zlib, raw Deflate, or Brotli implementation. Without `format`, it identifies Gzip and Deflate from their signatures; Brotli and raw Deflate have none and need `format`.

The limits are checked while the output is produced, never after, so a decompression bomb stops at the limit instead of filling the memory. `maxRatio` applies only after the first 1 MiB of output, since small inputs legitimately expand by large factors.

<SignatureBlock label="Signature">decompress(data: u8[], options?: DecompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/compress/decompress/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/compress/decompress/maximal.md -->

A `format` that does not match the data fails with `ERR_COMPRESS_INVALID_DATA`, and a limit of `0` fails with `ERR_COMPRESS_INVALID_OPTION`: options raise the limits, they never switch them off.
