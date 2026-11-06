---
title: decompress
description: Descomprime bytes con seguridad, detectando Gzip y Deflate por los datos.
---

# decompress

`decompress` restaura bytes producidos por `compress` o por cualquier otra implementación de Gzip, zlib, Deflate crudo o Brotli. Sin `format`, identifica Gzip y Deflate por sus firmas; Brotli y Deflate crudo no tienen firma y necesitan `format`.

Los límites se comprueban mientras se produce la salida, nunca después, así que una bomba de descompresión se detiene en el límite en lugar de llenar la memoria. `maxRatio` solo se aplica después del primer 1 MiB de salida, ya que las entradas pequeñas se expanden legítimamente por factores grandes.

<SignatureBlock label="Firma">decompress(data: u8[], options?: DecompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/decompress/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/compress/decompress/maximal.md -->

Un `format` que no coincide con los datos falla con `ERR_COMPRESS_INVALID_DATA`, y un límite `0` falla con `ERR_COMPRESS_INVALID_OPTION`: las opciones aumentan los límites, nunca los desactivan.
