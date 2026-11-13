---
title: compressionStream
description: Comprime un stream de bytes de cualquier tamaño con memoria constante.
---

# compressionStream

`compressionStream` devuelve un `TransformStream<u8[], u8[]>`: los bytes escritos en su lado `writable` salen comprimidos por su lado `readable`, en el formato indicado. Nunca guarda la entrada ni la salida completas, así que un archivo de cualquier tamaño se comprime con memoria constante, y un lector lento frena al escritor en lugar de dejar que los datos se acumulen.

<SignatureBlock label="Firma">compressionStream(format: CompressFormat, options?: CompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/compression-stream/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/compress/compression-stream/maximal.md -->

## Errores

Un fallo llega a ambos lados como un [`CompressError`](./types.md#compresserror): `read()` en la salida y `write()` o `close()` en la entrada rechazan con él, y `catch (error: CompressError)` lo recibe. Un `level` fuera del rango del formato hace que el stream falle en su primer uso con `ERR_COMPRESS_INVALID_LEVEL`; uno literal se rechaza al compilar (`E3312`).
