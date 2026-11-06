---
title: writeFileSync
description: Escreve arquivos UTF-8 de forma síncrona pelo módulo nativo tyrus:fs.
---

# writeFileSync

`writeFileSync` escreve dados em um arquivo. Se o arquivo não existir, é criado; se existir, é sobrescrito.

<SignatureBlock label="Assinatura">writeFileSync(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/write-file-sync/basic-usage.md -->
## Tratamento de erros

Se o arquivo não puder ser escrito, `writeFileSync` retorna `Err(FsError)` com um campo `code` estável. Inspecione o resultado com `match` ou propague com `?`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/write-file-sync/error-handling.md -->
:::

## Copiar um arquivo

<!-- @include: ../../../.fragments/modules/fs/write-file-sync/copy.md -->
