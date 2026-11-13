---
title: readFileSync
description: Lee archivos UTF-8 de forma síncrona mediante el módulo nativo tyrus:fs.
---

# readFileSync

`readFileSync` devuelve el contenido UTF-8 del archivo en `Ok(string)`, o un `FsError` en `Err`.

<SignatureBlock label="Firma">readFileSync(path: string): Result&lt;string, FsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/read-file-sync/basic-usage.md -->
## Tratamiento de errores

Si el archivo no puede leerse, `readFileSync` devuelve `Err(FsError)` con un campo `code` estable (ej: `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) y un `message` legible. Inspeccione el resultado con `match` o propáguelo con `?`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/read-file-sync/error-handling.md -->
:::

## Con tipos explícitos

<!-- @include: ../../../.fragments/modules/fs/read-file-sync/with-explicit-types.md -->
