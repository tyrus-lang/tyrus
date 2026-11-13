---
title: writeFileSync
description: Escribe archivos UTF-8 de forma síncrona mediante el módulo nativo tyrus:fs.
---

# writeFileSync

`writeFileSync` escribe datos en un archivo. Si el archivo no existe, se crea; si existe, se sobrescribe.

<SignatureBlock label="Firma">writeFileSync(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/write-file-sync/basic-usage.md -->
## Tratamiento de errores

Si el archivo no puede escribirse, `writeFileSync` devuelve `Err(FsError)` con un campo `code` estable. Inspeccione el resultado con `match` o propáguelo con `?`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/write-file-sync/error-handling.md -->
:::

## Copiar un archivo

<!-- @include: ../../../.fragments/modules/fs/write-file-sync/copy.md -->
