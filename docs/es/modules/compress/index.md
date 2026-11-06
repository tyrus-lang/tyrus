---
title: tyrus:compress
description: Comprime y descomprime bytes con Gzip, Deflate, Deflate crudo y Brotli.
---

# tyrus:compress

`tyrus:compress` comprime y descomprime buffers y streams de bytes con Gzip, Deflate (zlib), Deflate crudo y Brotli. Toda operación es asíncrona y se ejecuta fuera de los hilos que ejecutan las funciones `async` del propio programa, así que un buffer grande nunca los bloquea.

La descompresión es segura por defecto: el tamaño de la salida y la razón de expansión se comprueban mientras se producen los datos, así que una entrada pequeña y maliciosa no puede llenar la memoria. Las opciones solo aumentan estos límites, nunca los desactivan.

## Imports

<SignatureBlock label="Importaciones">import { CompressError, CompressFormat, CompressOptions, DecompressOptions, compress, compressionStream, decompress, decompressionStream } from "tyrus:compress";</SignatureBlock>

## Funciones

| Item | Firma | Propósito |
|------|-----------|---------|
| [`compress`](./compress.md) | `(data: u8[] \| string, format: CompressFormat, options?: CompressOptions) => Promise<Result<u8[], CompressError>>` | Comprime bytes o los bytes UTF-8 de una cadena. |
| [`decompress`](./decompress.md) | `(data: u8[], options?: DecompressOptions) => Promise<Result<u8[], CompressError>>` | Descomprime bytes, detectando Gzip y Deflate por los datos. |
| [`compressionStream`](./compression-stream.md) | `(format: CompressFormat, options?: CompressOptions) => TransformStream<u8[], u8[]>` | Comprime un stream de bytes de cualquier tamaño con memoria constante. |
| [`decompressionStream`](./decompression-stream.md) | `(options?: DecompressOptions) => TransformStream<u8[], u8[]>` | Descomprime un stream de bytes de cualquier tamaño con memoria constante. |

## Formatos

| `CompressFormat` | Formato | Detectado por `decompress` | Niveles |
|------------------|--------|--------------------------|--------|
| `Gzip` | Gzip (`.gz`), incluidos varios miembros concatenados | Sí | `0` a `9`, por defecto `6` |
| `Deflate` | zlib, el `deflate` del encabezado HTTP `Content-Encoding` | Sí | `0` a `9`, por defecto `6` |
| `DeflateRaw` | Deflate sin encabezado ni checksum | No | `0` a `9`, por defecto `6` |
| `Brotli` | Brotli | No | `0` a `11`, por defecto `6` |

La misma entrada, formato y nivel siempre producen los mismos bytes. Brotli y Deflate crudo no tienen firma, así que `decompress` necesita `format` para ellos en lugar de adivinar.

## Códigos de error

Toda función se resuelve con un `Result` y nunca rechaza: el éxito es `Ok` con los bytes, y un fallo es `Err` con un [`CompressError`](./types.md#compresserror). Su `code` es estable y su `message` es legible. Dentro de una función que retorna un `Result` con el mismo error, `await compress(...)?` entrega los bytes o retorna el error, y `onfail` lo trata en el mismo lugar.

| Código | Significado |
|------|---------|
| `ERR_COMPRESS_INVALID_DATA` | Los datos están malformados, truncados, tienen bytes después del final o no están en el formato indicado. |
| `ERR_COMPRESS_CHECKSUM_MISMATCH` | Los datos Gzip o zlib no coinciden con su checksum o con el tamaño registrado. |
| `ERR_COMPRESS_UNKNOWN_FORMAT` | Se omitió `format` y los datos no tienen firma Gzip ni zlib. |
| `ERR_COMPRESS_INVALID_LEVEL` | `level` está fuera del rango del formato. |
| `ERR_COMPRESS_INVALID_OPTION` | Una opción tiene un valor inválido, como un límite `0`. |
| `ERR_COMPRESS_OUTPUT_LIMIT` | La salida descomprimida superaría `maxOutputBytes`. |
| `ERR_COMPRESS_RATIO_LIMIT` | La salida descomprimida superaría `maxRatio` veces la entrada. |
| `ERR_COMPRESS_ALLOCATION_FAILED` | No se pudo reservar memoria para la salida. |

## Ejemplo

<!-- @include: ../../../.fragments/modules/compress/index/example.md -->
