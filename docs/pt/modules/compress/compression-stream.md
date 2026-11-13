---
title: compressionStream
description: Comprime uma stream de bytes de qualquer tamanho com memória constante.
---

# compressionStream

`compressionStream` retorna um `TransformStream<u8[], u8[]>`: os bytes escritos no lado `writable` saem comprimidos pelo lado `readable`, no formato informado. Ela nunca guarda a entrada nem a saída inteiras, então um arquivo de qualquer tamanho é comprimido com memória constante, e um leitor lento segura o escritor em vez de deixar os dados se acumularem.

<SignatureBlock label="Assinatura">compressionStream(format: CompressFormat, options?: CompressOptions): TransformStream&lt;u8[], u8[]&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/compress/compression-stream/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/compress/compression-stream/maximal.md -->

## Erros

Uma falha chega aos dois lados como um [`CompressError`](./types.md#compresserror): `read()` na saída e `write()` ou `close()` na entrada rejeitam com ele, e `catch (error: CompressError)` o recebe. Um `level` fora do intervalo do formato faz a stream falhar no primeiro uso com `ERR_COMPRESS_INVALID_LEVEL`; um literal é recusado na compilação (`E3312`).
