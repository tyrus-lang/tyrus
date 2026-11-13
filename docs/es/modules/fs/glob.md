---
title: glob
description: Encuentra archivos que coinciden con un patrón glob.
---

# glob

Compara nombres de archivos y directorios con un patrón glob. El patrón se resuelve en relación con el directorio de trabajo del proceso antes de comparar. Los patrones relativos devuelven rutas relativas, mientras que los patrones absolutos devuelven rutas absolutas. Admite `*` (coincide con cualquier carácter excepto `/`) y `**` (coincide con cualquier cantidad de directorios).

<SignatureBlock label="Firma">glob(pattern: string): Result&lt;string[], FsError&gt;</SignatureBlock>

## Manejo de errores

Devuelve `Err(FsError)` con el código `ERR_FS_GLOB_FAILED` si no se puede listar un directorio o inspeccionar una entrada necesaria para el recorrido. Un recorrido fallido no devuelve coincidencias parciales; si no hay coincidencias, el resultado es `Ok([])`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/glob/example.md -->
