---
title: unpackStream
description: Extrai um arquivo TAR ou TAR.GZ de uma stream de bytes à medida que chega.
---

# unpackStream

`unpackStream` extrai um arquivo TAR ou TAR.GZ de uma `ReadableStream<u8[]>` para um diretório enquanto os bytes chegam, com a mesma segurança de [`unpack`](./unpack.md): as entradas ficam dentro do destino, links e arquivos especiais são recusados, os limites valem e, numa falha, tudo o que a chamada criou é removido. O formato é identificado pelos primeiros bytes.

Um arquivo ZIP não pode ser extraído de uma stream: sua tabela de entradas fica no fim, e as entradas antes dela não são confiáveis até ela ser lida. Uma stream assim falha com `ERR_ARCHIVE_ZIP_NOT_STREAMABLE`; grave-a num arquivo e use `unpack`.

<SignatureBlock label="Assinatura">unpackStream(archive: ReadableStream&lt;u8[]&gt;, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/unpack-stream/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/archive/unpack-stream/maximal.md -->

A stream é emprestada: a chamada a lê até o fim do arquivo, e o programa mantém seu handle. Uma stream que falha durante a leitura resolve com `ERR_ARCHIVE_IO`, e uma em que ninguém escreve falha em `timeoutMs`.
