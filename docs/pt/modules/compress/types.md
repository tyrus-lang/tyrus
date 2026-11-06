---
title: Tipos de tyrus:compress
description: Tipos de formato, opções e erro exportados por tyrus:compress.
---

# Tipos de tyrus:compress

::: tip Tipos precisam ser importados pelo nome
`CompressError`, `CompressFormat`, `CompressOptions` e `DecompressOptions` precisam ser importados explicitamente sempre que seus campos, membros ou literais de objeto forem usados.
:::

## CompressFormat

| Membro | Formato |
|--------|--------|
| `Gzip` | Gzip, RFC 1952. |
| `Deflate` | zlib, RFC 1950. |
| `DeflateRaw` | Deflate cru, RFC 1951. |
| `Brotli` | Brotli, RFC 7932. |

Veja os [formatos](./index.md#formatos) para níveis e detecção.

## CompressOptions

| Campo | Tipo | Padrão |
|-------|------|---------|
| `level` | `i32?` | `6` |

## DecompressOptions

| Campo | Tipo | Padrão |
|-------|------|---------|
| `format` | `CompressFormat?` | detectado pelos dados |
| `maxOutputBytes` | `u64?` | `256 MiB` em `decompress`, `4 GiB` em `decompressionStream` |
| `maxRatio` | `u32?` | `200` |

Veja [`decompress`](./decompress.md#todas-as-opcoes) e [`decompressionStream`](./decompression-stream.md#todas-as-opcoes) para o que cada opção faz.

## CompressError

`CompressError` é o erro recuperável de toda operação, entregue como o `Err` do `Result` com que cada função resolve.

| Campo | Tipo | Descrição |
|-------|------|-------------|
| `code` | `string` | Identificador estável, como `ERR_COMPRESS_RATIO_LIMIT`. Veja os [códigos de erro](./index.md#codigos-de-erro). |
| `message` | `string` | Descrição legível. |

<!-- @include: ../../../.fragments/modules/compress/types/error.md -->
