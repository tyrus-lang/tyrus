---
title: readFile
description: Lee archivos UTF-8 de forma asíncrona mediante el módulo nativo tyrus:fs.
---

# readFile

`readFile` lee un archivo de forma asíncrona y devuelve un `Promise<Result<string, FsError>>` con el contenido UTF-8 del archivo.

<SignatureBlock label="Firma">readFile(path: string): Promise&lt;Result&lt;string, FsError&gt;&gt;</SignatureBlock>

<!-- @include: ../../../.fragments/modules/fs/read-file/basic-usage.md -->

## Tratamiento de errores

Si el archivo no puede leerse, la promise se cumple con `Err(FsError)`. El error tiene un campo `code` estable (p. ej., `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) y un `message` legible. Trata el resultado con `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/read-file/error-handling.md -->
:::

## Con tipos explícitos

<!-- @include: ../../../.fragments/modules/fs/read-file/with-explicit-types.md -->
