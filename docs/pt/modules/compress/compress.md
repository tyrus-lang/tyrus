---
title: compress
description: Comprime bytes ou texto com Gzip, Deflate, Deflate cru ou Brotli.
---

# compress

`compress` comprime um `u8[]`, ou os bytes UTF-8 de uma `string`, no formato informado. O formato é sempre explícito: a mesma chamada com a mesma entrada produz os mesmos bytes onde quer que rode.

<SignatureBlock label="Assinatura">compress(data: u8[] | string, format: CompressFormat, options?: CompressOptions): Promise&lt;Result&lt;u8[], CompressError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/compress/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/compress/compress/maximal.md -->

Um `level` literal fora do intervalo de um membro literal de `CompressFormat` é recusado na compilação (`E3312`); um nível calculado em tempo de execução falha com `ERR_COMPRESS_INVALID_LEVEL`.
