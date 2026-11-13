---
title: Tipos de tyrus:compress
description: Tipos de formato, opciones y error exportados por tyrus:compress.
---

# Tipos de tyrus:compress

::: tip Los tipos deben importarse por nombre
`CompressError`, `CompressFormat`, `CompressOptions` y `DecompressOptions` deben importarse explícitamente siempre que se usen sus campos, miembros o literales de objeto.
:::

## CompressFormat

| Miembro | Formato |
|--------|--------|
| `Gzip` | Gzip, RFC 1952. |
| `Deflate` | zlib, RFC 1950. |
| `DeflateRaw` | Deflate crudo, RFC 1951. |
| `Brotli` | Brotli, RFC 7932. |

Consulta los [formatos](./index.md#formatos) para niveles y detección.

## CompressOptions

| Campo | Tipo | Por defecto |
|-------|------|---------|
| `level` | `i32?` | `6` |

## DecompressOptions

| Campo | Tipo | Por defecto |
|-------|------|---------|
| `format` | `CompressFormat?` | detectado por los datos |
| `maxOutputBytes` | `u64?` | `256 MiB` en `decompress`, `4 GiB` en `decompressionStream` |
| `maxRatio` | `u32?` | `200` |

Consulta [`decompress`](./decompress.md#todas-las-opciones) y [`decompressionStream`](./decompression-stream.md#todas-las-opciones) para lo que hace cada opción.

## CompressError

`CompressError` es el error recuperable de toda operación, entregado como el `Err` del `Result` con que se resuelve cada función.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `code` | `string` | Identificador estable, como `ERR_COMPRESS_RATIO_LIMIT`. Consulta los [códigos de error](./index.md#codigos-de-error). |
| `message` | `string` | Descripción legible. |

<!-- @include: ../../../.fragments/modules/compress/types/error.md -->
