---
title: readFile
description: Lê arquivos UTF-8 de forma assíncrona pelo módulo nativo tyrus:fs.
---

# readFile

`readFile` lê um arquivo de forma assíncrona e retorna um `Promise<Result<string, FsError>>` com o conteúdo UTF-8 do arquivo.

<SignatureBlock label="Assinatura">readFile(path: string): Promise&lt;Result&lt;string, FsError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/fs/read-file/basic-usage.md -->

## Tratamento de erros

Se o arquivo não puder ser lido, a promise é cumprida com `Err(FsError)`. O erro possui um campo `code` estável (ex.: `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) e uma `message` legível. Trate o resultado com `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/read-file/error-handling.md -->
:::

## Com tipos explícitos

<!-- @include: ../../../.fragments/modules/fs/read-file/with-explicit-types.md -->
