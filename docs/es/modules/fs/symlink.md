---
title: symlink
description: Crea un enlace simbólico.
---

# symlink

Crea un enlace simbólico en `link` que apunta a `original`. En Unix, `original` puede ser un archivo o un directorio. En Windows, esta operación crea enlaces simbólicos a archivos; usa [`symlinkDir`](./symlink-dir.md) para directorios. Crear un enlace simbólico en Windows puede requerir el Modo de desarrollador, el privilegio `SeCreateSymbolicLinkPrivilege` o un proceso elevado; consulta la [documentación de Microsoft para `CreateSymbolicLinkW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). Los fallos se devuelven como `Err(FsError)`.

<SignatureBlock label="Firma">symlink(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/symlink/example.md -->
