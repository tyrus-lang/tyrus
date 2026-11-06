---
title: tyrus:compress types
description: Format, option, and error types exported by tyrus:compress.
---

# tyrus:compress types

::: tip Types must be imported by name
`CompressError`, `CompressFormat`, `CompressOptions`, and `DecompressOptions` must be imported explicitly whenever their fields, members, or object literals are used.
:::

## CompressFormat

| Member | Format |
|--------|--------|
| `Gzip` | Gzip, RFC 1952. |
| `Deflate` | zlib, RFC 1950. |
| `DeflateRaw` | Raw Deflate, RFC 1951. |
| `Brotli` | Brotli, RFC 7932. |

See the [formats](./index.md#formats) for levels and detection.

## CompressOptions

| Field | Type | Default |
|-------|------|---------|
| `level` | `i32?` | `6` |

## DecompressOptions

| Field | Type | Default |
|-------|------|---------|
| `format` | `CompressFormat?` | detected from the data |
| `maxOutputBytes` | `u64?` | `256 MiB` for `decompress`, `4 GiB` for `decompressionStream` |
| `maxRatio` | `u32?` | `200` |

See [`decompress`](./decompress.md#every-option) and [`decompressionStream`](./decompression-stream.md#every-option) for what each option does.

## CompressError

`CompressError` is the recoverable error of every operation, delivered as the `Err` of the `Result` each function resolves with.

| Field | Type | Description |
|-------|------|-------------|
| `code` | `string` | Stable identifier, such as `ERR_COMPRESS_RATIO_LIMIT`. See the [error codes](./index.md#error-codes). |
| `message` | `string` | Human-readable description. |

<!-- @include: ../../.fragments/modules/compress/types/error.md -->
