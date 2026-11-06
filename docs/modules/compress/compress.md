---
title: compress
description: Compress bytes or text with Gzip, Deflate, raw Deflate, or Brotli.
---

# compress

`compress` compresses a `u8[]`, or the UTF-8 bytes of a `string`, in the format you name. The format is always explicit: the same call with the same input produces the same bytes wherever it runs.

<SignatureBlock label="Signature">compress(data: u8[] | string, format: CompressFormat, options?: CompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Minimal example

<!-- @include: ../../.fragments/modules/compress/compress/minimal.md -->

## Every option

<!-- @include: ../../.fragments/modules/compress/compress/maximal.md -->

A literal `level` outside the range of a literal `CompressFormat` member is refused when the program is compiled (`E3312`); a level computed at run time fails with `ERR_COMPRESS_INVALID_LEVEL`.
