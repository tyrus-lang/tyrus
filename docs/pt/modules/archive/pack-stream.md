---
title: packStream
description: Escreve um arquivo numa stream de bytes à medida que a origem é lida.
---

# packStream

`packStream` retorna uma `ReadableStream<u8[]>` com o arquivo de um arquivo ou diretório à medida que ele é escrito, no formato informado: nada é gravado em disco, então o arquivo pode ir direto para uma resposta HTTP, uma stream de compressão ou um arquivo. A stream guarda no máximo 64 KiB, e o arquivo só é produzido na velocidade em que é lido.

O ZIP é escrito com data descriptors depois de cada entrada, já que uma stream não pode voltar para preencher tamanhos; toda ferramenta atual o lê.

<SignatureBlock label="Assinatura">packStream(source: string, format: ArchiveFormat, options?: PackOptions): ReadableStream&lt;u8[]&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/pack-stream/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/archive/pack-stream/maximal.md -->

`overwrite` não tem efeito, já que não há destino. Uma falha, de uma origem inexistente ao limite de tempo, faz a stream falhar com o [`ArchiveError`](./types.md#archiveerror) em vez de terminá-la, então um leitor nunca confunde um arquivo truncado com um completo: `read()` e `pipeTo()` rejeitam, e `catch (error: ArchiveError)` o recebe.
