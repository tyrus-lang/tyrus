---
title: readlink
description: Lee el destino de un enlace simbólico.
---

# readlink

Devuelve la ruta de destino a la que apunta un enlace simbólico o `ERR_FS_READLINK_FAILED` si la ruta no es un enlace simbólico legible. Una ruta no válida devuelve `ERR_FS_INVALID_PATH`.

<SignatureBlock label="Firma">readlink(path: string): Result&lt;string, FsError&gt;</SignatureBlock>

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/readlink/example.md -->
