---
title: symlinkDir
description: Crea un enlace simbólico a un directorio.
---

# symlinkDir

Crea un enlace simbólico en `link` que apunta al directorio en `original`. Esta operación identifica el enlace como un enlace a directorio incluso cuando el destino todavía no existe. En Windows, crear un enlace simbólico puede requerir el Modo de desarrollador, el privilegio `SeCreateSymbolicLinkPrivilege` o un proceso elevado; consulta la [documentación de Microsoft para `CreateSymbolicLinkW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createsymboliclinkw). Los fallos se devuelven como `Err(FsError)`.

<SignatureBlock label="Firma">symlinkDir(original: string, link: string): Result&lt;void, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/symlink-dir/example.md -->
