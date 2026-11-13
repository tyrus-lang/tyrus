---
title: pack
description: Crea un archivo ZIP, TAR o TAR.GZ a partir de un archivo o directorio.
---

# pack

`pack` crea un archivo a partir de un archivo o de un directorio. La extensión del destino, en minúsculas, decide el formato: `.zip`, `.tar`, `.tar.gz` o `.tgz`.

Las entradas se escriben en un orden estable con separadores `/`, y el archivo se escribe primero en un archivo temporal junto al destino y luego se renombra a su nombre final, así que una falla nunca deja un archivo truncado. Cuando el destino está dentro del directorio de origen, queda fuera del archivo.

<SignatureBlock label="Firma">pack(source: string, destination: string, options?: PackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/pack/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/archive/pack/maximal.md -->

| Opción | Tipo | Por defecto | Efecto |
|--------|------|---------|--------|
| `level` | `i32` | `6` | Nivel de compresión de `0` (solo almacena) a `9` (salida más pequeña). No se acepta para `.tar`. |
| `overwrite` | `boolean` | `false` | Reemplaza un archivo de destino existente. Un directorio nunca se reemplaza. |
| `exclude` | `string[]` | ninguno | Patrones glob, relativos al origen, de archivos y directorios a dejar fuera. `**` abarca cualquier cantidad de directorios. |
| `prefix` | `string` | ninguno | Directorio que contiene todas las entradas, como `release-1.0.0`. |
| `reproducible` | `boolean` | `false` | Escribe fechas fijas y permisos normalizados, para que la misma entrada produzca los mismos bytes. |
| `timeoutMs` | `u64` | `600000` (10 minutos) | Desiste con `ERR_ARCHIVE_TIMEOUT` cuando la creación tarda más, sin dejar un archivo parcial. `0` falla con `ERR_ARCHIVE_INVALID_OPTION`. |

Los enlaces simbólicos y archivos especiales del origen se rechazan con `ERR_ARCHIVE_UNSUPPORTED_ENTRY`, que indica la ruta en `entry`, en lugar de omitirse en silencio.

## Manejo de errores

::: danger error
<!-- @include: ../../../.fragments/modules/archive/pack/error-handling.md -->
:::

Un destino literal con una extensión desconocida o en mayúsculas, y un `level` literal que el formato no acepta, los señala el compilador antes de que el programa se ejecute.
