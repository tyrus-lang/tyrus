---
title: decompressionStream
description: Descomprime uma stream de bytes com segurança e memória constante.
---

# decompressionStream

`decompressionStream` retorna um `TransformStream<u8[], u8[]>` que restaura dados Gzip, zlib, Deflate cru ou Brotli à medida que chegam. Sem `format`, identifica Gzip e Deflate pelos primeiros bytes; Brotli e Deflate cru precisam de `format`.

A saída sai em pedaços de no máximo 64 KiB, e cada um espera espaço na saída antes que o próximo seja produzido, então a memória fica constante mesmo quando uma entrada pequena se expande enormemente. Os limites valem para a stream inteira: passado `maxOutputBytes` ou `maxRatio`, a stream falha em vez de produzir mais.

<SignatureBlock label="Assinatura">decompressionStream(options?: DecompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/decompression-stream/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/compress/decompression-stream/maximal.md -->

## Erros

Uma falha chega aos dois lados como um [`CompressError`](./types.md#compresserror), com os mesmos [códigos](./index.md#codigos-de-erro) de `decompress`: dados que terminam antes da hora ou têm bytes depois do fim são `ERR_COMPRESS_INVALID_DATA`, e os limites são `ERR_COMPRESS_OUTPUT_LIMIT` e `ERR_COMPRESS_RATIO_LIMIT`. O `maxOutputBytes` padrão de uma stream é 4 GiB, já que uma stream nunca guarda sua saída.
