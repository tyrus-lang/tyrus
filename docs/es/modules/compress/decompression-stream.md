---
title: decompressionStream
description: Descomprime un stream de bytes con seguridad y memoria constante.
---

# decompressionStream

`decompressionStream` devuelve un `TransformStream<u8[], u8[]>` que restaura datos Gzip, zlib, Deflate crudo o Brotli a medida que llegan. Sin `format`, identifica Gzip y Deflate por los primeros bytes; Brotli y Deflate crudo necesitan `format`.

La salida sale en trozos de como máximo 64 KiB, y cada uno espera espacio en la salida antes de que se produzca el siguiente, así que la memoria se mantiene constante incluso cuando una entrada pequeña se expande enormemente. Los límites se aplican al stream completo: superados `maxOutputBytes` o `maxRatio`, el stream falla en lugar de producir más.

<SignatureBlock label="Firma">decompressionStream(options?: DecompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/decompression-stream/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/compress/decompression-stream/maximal.md -->

## Errores

Un fallo llega a ambos lados como un [`CompressError`](./types.md#compresserror), con los mismos [códigos](./index.md#codigos-de-error) que `decompress`: los datos que terminan antes de tiempo o tienen bytes después del final son `ERR_COMPRESS_INVALID_DATA`, y los límites son `ERR_COMPRESS_OUTPUT_LIMIT` y `ERR_COMPRESS_RATIO_LIMIT`. El `maxOutputBytes` por defecto de un stream es 4 GiB, ya que un stream nunca guarda su salida.
