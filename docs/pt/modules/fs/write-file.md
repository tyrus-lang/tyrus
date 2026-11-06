---
title: writeFile
description: Escreve arquivos UTF-8 de forma assíncrona pelo módulo nativo tyrus:fs.
---

# writeFile

`writeFile` escreve dados em um arquivo de forma assíncrona. Se o arquivo não existir, é criado; se existir, é sobrescrito.

<SignatureBlock label="Assinatura">writeFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/write-file/basic-usage.md -->
## Tratamento de erros

Se o arquivo não puder ser escrito, a promise é cumprida com `Err(FsError)`. Trate o resultado com `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/write-file/error-handling.md -->
:::

## Copiar um arquivo

<!-- @include: ../../../.fragments/modules/fs/write-file/copy.md -->
