---
title: chown
description: Cambia el propietario de un archivo.
---

# chown

Cambia el propietario y el grupo de un archivo o directorio. En plataformas que no son Unix, devuelve `Err` con el código `ERR_FS_CHOWN_FAILED`.

<SignatureBlock label="Firma">chown(path: string, uid: u32, gid: u32): Result&lt;void, FsError&gt;</SignatureBlock>

## Parámetros

- `path`: ruta del archivo que se cambiará.
- `uid`: nuevo ID de usuario.
- `gid`: nuevo ID de grupo.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/chown/example.md -->
