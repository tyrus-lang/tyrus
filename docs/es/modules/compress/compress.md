---
title: compress
description: Comprime bytes o texto con Gzip, Deflate, Deflate crudo o Brotli.
---

# compress

`compress` comprime un `u8[]`, o los bytes UTF-8 de una `string`, en el formato indicado. El formato siempre es explícito: la misma llamada con la misma entrada produce los mismos bytes dondequiera que se ejecute.

<SignatureBlock label="Firma">compress(data: u8[] | string, format: CompressFormat, options?: CompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/compress/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/compress/compress/maximal.md -->

Un `level` literal fuera del rango de un miembro literal de `CompressFormat` se rechaza al compilar (`E3312`); un nivel calculado en tiempo de ejecución falla con `ERR_COMPRESS_INVALID_LEVEL`.
