---
title: decompress
description: Descomprime bytes com segurança, detectando Gzip e Deflate pelos dados.
---

# decompress

`decompress` restaura bytes produzidos por `compress` ou por qualquer outra implementação de Gzip, zlib, Deflate cru ou Brotli. Sem `format`, identifica Gzip e Deflate pelas assinaturas; Brotli e Deflate cru não têm assinatura e precisam de `format`.

Os limites são verificados enquanto a saída é produzida, nunca depois, então uma bomba de descompressão para no limite em vez de encher a memória. `maxRatio` só vale depois do primeiro 1 MiB de saída, já que entradas pequenas legitimamente se expandem por fatores grandes.

<SignatureBlock label="Assinatura">decompress(data: u8[], options?: DecompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/decompress/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/compress/decompress/maximal.md -->

Um `format` que não confere com os dados falha com `ERR_COMPRESS_INVALID_DATA`, e um limite `0` falha com `ERR_COMPRESS_INVALID_OPTION`: as opções aumentam os limites, nunca os desligam.
