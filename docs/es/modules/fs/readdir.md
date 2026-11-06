---
title: readdir
description: Lee el contenido de un directorio.
---

# readdir

Lee el contenido de un directorio y devuelve un array de objetos `DirEntry`.

<SignatureBlock label="Firma">readdir(path: string, options?: ReaddirOptions): Result&lt;DirEntry[], FsError&gt;</SignatureBlock>

## Opciones de lectura

`ReaddirOptions` es una interfaz con un campo opcional:

- `recursive?: boolean`: cuando es `true`, recorre los subdirectorios de forma recursiva e incluye sus entradas en el resultado. El valor predeterminado es `false`.

## Entradas de directorio

Cada `DirEntry` tiene los campos de solo lectura `name: string` y `type: DirEntryType`. `DirEntryType` es un enum con los miembros `File`, `Directory`, `Symlink` y `Other`, que permite identificar el tipo de entrada sin inspeccionar su nombre.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/readdir/basic-usage.md -->

## Tratamiento de errores

Devuelve `Err(FsError)` con el código `ERR_FS_READ_FAILED` si no se puede leer el directorio raíz o algún directorio recorrido de forma recursiva, o si no se puede inspeccionar una entrada. La operación no devuelve un listado parcial.

::: danger error
<!-- @include: ../../../.fragments/modules/fs/readdir/error-handling.md -->
:::
