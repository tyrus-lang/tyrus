---
title: readFileSync
description: Lê arquivos UTF-8 de forma síncrona pelo módulo nativo tyrus:fs.
---

# readFileSync

`readFileSync` retorna o conteúdo UTF-8 do arquivo em `Ok(string)` ou um `FsError` em `Err`.

<SignatureBlock label="Assinatura">readFileSync(path: string): Result&lt;string, FsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/read-file-sync/basic-usage.md -->
## Tratamento de erros

Se o arquivo não puder ser lido, `readFileSync` retorna `Err(FsError)` com um campo `code` estável (ex: `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) e uma `message` legível. Inspecione o resultado com `match` ou propague com `?`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/read-file-sync/error-handling.md -->
:::

## Com tipos explícitos

<!-- @include: ../../../.fragments/modules/fs/read-file-sync/with-explicit-types.md -->
