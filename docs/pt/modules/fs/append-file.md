---
title: appendFile
description: Acrescenta dados a um arquivo de forma assíncrona pelo módulo nativo tyrus:fs.
---

# appendFile

`appendFile` acrescenta dados em um arquivo de forma assíncrona. Se o arquivo não existir, é criado.

<SignatureBlock label="Assinatura">appendFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/append-file/basic-usage.md -->
## Tratamento de erros

Se não for possível acrescentar dados ao arquivo, a promise é cumprida com `Err(FsError)`. Trate o resultado com `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/append-file/error-handling.md -->
:::
