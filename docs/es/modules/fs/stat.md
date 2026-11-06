---
title: stat
description: Obtiene estadísticas de un archivo.
---

# stat

Devuelve estadísticas de un archivo o directorio. Las rutas no válidas devuelven `Err` con `ERR_FS_INVALID_PATH`; las rutas que no se pueden inspeccionar devuelven `Err` con `ERR_FS_STAT_FAILED`.

Si `path` es un enlace simbólico, las estadísticas describen el destino e `isSymlink` es `true` para la ruta original.

<SignatureBlock label="Firma">stat(path: string): Result&lt;FileStats, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `size` | `u64` | Tamaño del archivo en bytes. |
| `isFile` | `boolean` | `true` si la ruta es un archivo. |
| `isDirectory` | `boolean` | `true` si la ruta es un directorio. |
| `isSymlink` | `boolean` | `true` si la ruta es un enlace simbólico. |
| `modified` | `u64` | Hora de la última modificación (segundos desde la época Unix). |
| `created` | `u64` | Hora de creación (segundos desde la época Unix). |
| `permissions` | `u32` | Bits de permisos del archivo. |

Para una ruta accesible en plataformas que no son Unix, `permissions` vale `0`; esta API expone bits de permisos POSIX y no convierte las ACL de Windows.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/stat/example.md -->
