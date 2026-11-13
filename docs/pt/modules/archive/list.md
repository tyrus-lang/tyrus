---
title: list
description: Lê as entradas de um arquivo sem extraí-lo.
---

# list

`list` lê as entradas de um arquivo ZIP, TAR ou TAR.GZ sem gravar nada em disco. O formato é identificado pelo conteúdo, nunca pelo nome do arquivo.

Use-o para inspecionar um arquivo não confiável antes de extraí-lo: entradas que o `unpack` recusaria, como links, aparecem com seu tipo, e os nomes são informados exatamente como o arquivo os guarda.

<SignatureBlock label="Assinatura">list(archive: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveEntry[], ArchiveError&gt;&gt;</SignatureBlock>

## Exemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/list/minimal.md -->

## Todas as opções

<!-- @include: ../../../.fragments/modules/archive/list/maximal.md -->

`list` lê apenas `maxEntries` e `timeoutMs` de [`UnpackOptions`](./types.md#unpackoptions); as demais opções se aplicam à extração. Um arquivo com mais entradas falha com `ERR_ARCHIVE_ENTRY_LIMIT`, e um arquivo ZIP falha antes de sua tabela de entradas ser lida.
