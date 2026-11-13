---
title: diff
description: Compara dos archivos o directorios.
---

# diff

Compara dos archivos o directorios y devuelve un `DiffResult` que describe las diferencias.

<SignatureBlock label="Firma">diff(path1: string, path2: string): Result&lt;DiffResult, FsError&gt;</SignatureBlock>

## Campos de DiffResult

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `added` | `string[]` | Archivos presentes en path1, pero no en path2. |
| `removed` | `string[]` | Archivos presentes en path2, pero no en path1. |
| `modified` | `string[]` | Archivos presentes en ambos, pero con contenido diferente. |
| `unchanged` | `string[]` | Archivos presentes en ambos con contenido idéntico. |

## Manejo de errores

Devuelve `Err(FsError)` con el código `ERR_FS_DIFF_FAILED` si no se puede inspeccionar alguna ruta, listar un directorio o leer un archivo coincidente. La operación no informa los errores de lectura como diferencias.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/diff/example.md -->
