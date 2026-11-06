---
title: appendFile
description: Añade datos a un archivo de forma asíncrona mediante el módulo nativo tyrus:fs.
---

# appendFile

`appendFile` añade datos a un archivo de forma asíncrona. Si el archivo no existe, se crea.

<SignatureBlock label="Firma">appendFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/fs/append-file/basic-usage.md -->
## Tratamiento de errores

Si no se pueden añadir datos al archivo, la promise se cumple con `Err(FsError)`. Trata el resultado con `match`:

::: danger error
<!-- @include: ../../../.fragments/modules/fs/append-file/error-handling.md -->
:::
