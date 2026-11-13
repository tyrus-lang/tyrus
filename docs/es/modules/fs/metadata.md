---
title: metadata
description: Obtiene metadatos detallados de archivos.
---

# metadata

Devuelve metadatos detallados de un archivo o directorio. Si `path` es un enlace simbólico, los campos de metadatos describen el destino e `isSymlink` indica que la ruta original es un enlace. Devuelve `Err(FsError)` si falla.

<SignatureBlock label="Firma">metadata(path: string): Result&lt;Metadata, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `size` | `u64` | Tamaño del archivo en bytes. |
| `isFile` | `boolean` | `true` si la ruta es un archivo. |
| `isDirectory` | `boolean` | `true` si la ruta es un directorio. |
| `isSymlink` | `boolean` | `true` si la ruta es un enlace simbólico. |
| `modified` | `u64` | Hora de la última modificación (segundos desde la época Unix). |
| `created` | `u64` | Hora de creación (segundos desde la época Unix). |
| `accessed` | `u64` | Hora del último acceso (segundos desde la época Unix). |
| `permissions` | `u32` | Bits de permisos del archivo. |
| `uid` | `u32` | ID de usuario del propietario del archivo. |
| `gid` | `u32` | ID del grupo propietario del archivo. |
| `mode` | `u32` | Tipo de archivo y modo de permisos. |

En plataformas que no son Unix, `permissions`, `uid`, `gid` y `mode` actualmente valen `0`. Estos campos exponen bits de permisos POSIX e ID de propietario Unix; las ACL de Windows no se convierten a esta representación.

## Tratamiento de errores

<!-- @include: ../../../.fragments/modules/fs/metadata/error-handling.md -->
