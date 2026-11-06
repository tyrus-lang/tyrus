---
title: tyrus:compress
description: Compress and decompress bytes with Gzip, Deflate, raw Deflate, and Brotli.
---

# tyrus:compress

`tyrus:compress` compresses and decompresses byte buffers and byte streams with Gzip, Deflate (zlib), raw Deflate, and Brotli. Every operation is asynchronous and runs off the threads that run the program's own `async` functions, so a large buffer never stalls them.

Decompression is safe by default: the output size and the expansion ratio are checked while the data is produced, so a small malicious input cannot fill the memory. Options only raise these limits, they never switch them off.

## Imports

<SignatureBlock label="Imports">import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, compressionStream, decompress, decompressionStream } from "tyrus:compress";</SignatureBlock>

## Functions

| Item | Signature | Purpose |
|------|-----------|---------|
| [`compress`](./compress.md) | `(data: u8[] \| string, format: CompressFormat, options?: CompressOptions) => Promise<Result<u8[], CompressError>>` | Compresses bytes or the UTF-8 bytes of a string. |
| [`decompress`](./decompress.md) | `(data: u8[], options?: DecompressOptions) => Promise<Result<u8[], CompressError>>` | Decompresses bytes, detecting Gzip and Deflate from the data. |
| [`compressionStream`](./compression-stream.md) | `(format: CompressFormat, options?: CompressOptions) => TransformStream<u8[], u8[]>` | Compresses a byte stream of any size with constant memory. |
| [`decompressionStream`](./decompression-stream.md) | `(options?: DecompressOptions) => TransformStream<u8[], u8[]>` | Decompresses a byte stream of any size with constant memory. |

## Formats

| `CompressFormat` | Format | Detected by `decompress` | Levels |
|------------------|--------|--------------------------|--------|
| `Gzip` | Gzip (`.gz`), including several members joined one after the other | Yes | `0` to `9`, default `6` |
| `Deflate` | zlib, the `deflate` of the HTTP `Content-Encoding` header | Yes | `0` to `9`, default `6` |
| `DeflateRaw` | Deflate without a header or checksum | No | `0` to `9`, default `6` |
| `Brotli` | Brotli | No | `0` to `11`, default `6` |

The same input, format, and level always produce the same bytes. Brotli and raw Deflate carry no signature, so `decompress` needs `format` for them instead of guessing.

## Error codes

Every function resolves with a `Result` and never rejects: success is `Ok` with the bytes, and a failure is `Err` with a [`CompressError`](./types.md#compresserror). Its `code` is stable and its `message` is readable. Inside a function that returns a `Result` with the same error, `await compress(...)?` hands the bytes over or returns the error, and `onfail` handles it in place.

| Code | Meaning |
|------|---------|
| `ERR_COMPRESS_INVALID_DATA` | The data is malformed, truncated, has bytes after its end, or is not in the given format. |
| `ERR_COMPRESS_CHECKSUM_MISMATCH` | The Gzip or zlib data does not match its checksum or its recorded size. |
| `ERR_COMPRESS_UNKNOWN_FORMAT` | `format` was omitted and the data carries no Gzip or zlib signature. |
| `ERR_COMPRESS_INVALID_LEVEL` | `level` is outside the range of the format. |
| `ERR_COMPRESS_INVALID_OPTION` | An option has an invalid value, such as a limit of `0`. |
| `ERR_COMPRESS_OUTPUT_LIMIT` | The decompressed output would exceed `maxOutputBytes`. |
| `ERR_COMPRESS_RATIO_LIMIT` | The decompressed output would exceed `maxRatio` times its input. |
| `ERR_COMPRESS_ALLOCATION_FAILED` | Memory could not be allocated for the output. |

## Example

<!-- @include: ../../.fragments/modules/compress/index/example.md -->
