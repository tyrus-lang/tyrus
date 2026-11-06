---
title: writeFile
description: Escribe archivos UTF-8 de forma asíncrona mediante el módulo nativo tyrus:fs.
---

# writeFile

`writeFile` escribe datos en un archivo de forma asíncrona. Si el archivo no existe, se crea; si existe, se sobrescribe.

<SignatureBlock label="Firma">writeFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/write-file/basic-usage.md -->
## Tratamiento de errores

Si el archivo no puede escribirse, la promise se cumple con `Err(FsError)`. Trata el resultado con `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/write-file/error-handling.md -->
:::

## Copiar un archivo

<!-- @include: ../../../.fragments/modules/fs/write-file/copy.md -->
