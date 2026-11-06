---
title: Tipos de tyrus:archive
description: Tipos de entrada, resumen, opciones y error exportados por tyrus:archive.
---

# Tipos de tyrus:archive

::: tip Los tipos deben importarse por nombre
`ArchiveEntry`, `ArchiveEntryKind`, `ArchiveError`, `ArchiveFormat`, `ArchiveSummary`, `PackOptions` y `UnpackOptions` deben importarse explícitamente siempre que se usen sus campos, miembros o literales de objeto.
:::

## ArchiveSummary

`ArchiveSummary` lo devuelven [`pack`](./pack.md) y [`unpack`](./unpack.md).

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `entries` | `u64` | Archivos y directorios escritos. |
| `bytes` | `u64` | Bytes de archivo sin comprimir escritos. |

## ArchiveEntry

`ArchiveEntry` lo devuelve [`list`](./list.md).

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `name` | `string` | El nombre de la entrada exactamente como el archivo lo guarda. Los nombres de directorio terminan en `/` en archivos ZIP. |
| `kind` | `ArchiveEntryKind` | Qué es la entrada. |
| `size` | `u64` | Tamaño sin comprimir en bytes. |
| `compressedSize` | `u64?` | Tamaño comprimido en bytes para entradas ZIP; ausente para entradas TAR. |

## ArchiveEntryKind

| Miembro | Significado |
|--------|---------|
| `File` | Un archivo regular. |
| `Directory` | Un directorio. |
| `Symlink` | Un enlace simbólico. `unpack` lo rechaza. |
| `HardLink` | Un enlace duro. `unpack` lo rechaza. |
| `Other` | Un dispositivo, pipe u otra entrada especial. `unpack` la rechaza. |

<!-- @include: ../../../.fragments/modules/archive/types/entry-kind.md -->

## ArchiveFormat

El formato que escribe [`packStream`](./pack-stream.md), ya que un stream no tiene extensión que lo decida.

| Miembro | Formato |
|--------|--------|
| `Zip` | ZIP, con data descriptors. |
| `Tar` | TAR, sin compresión. |
| `TarGz` | TAR comprimido con Gzip. |

## PackOptions

| Campo | Tipo | Por defecto |
|-------|------|---------|
| `level` | `i32?` | `6` |
| `overwrite` | `boolean?` | `false` |
| `exclude` | `string[]?` | ninguno |
| `prefix` | `string?` | ninguno |
| `reproducible` | `boolean?` | `false` |
| `timeoutMs` | `u64?` | `600000` |

Consulte en [`pack`](./pack.md#todas-las-opciones) qué hace cada opción.

## UnpackOptions

| Campo | Tipo | Por defecto |
|-------|------|---------|
| `overwrite` | `boolean?` | `false` |
| `stripComponents` | `u32?` | `0` |
| `maxEntries` | `u64?` | `100000` |
| `maxBytes` | `u64?` | `4 GiB` |
| `maxRatio` | `u32?` | `200` |
| `timeoutMs` | `u64?` | `600000` |

Consulte en [`unpack`](./unpack.md#todas-las-opciones) qué hace cada opción.

## ArchiveError

`ArchiveError` es el error recuperable de toda operación de archivo, entregado como el `Err` del `Result` con que resuelve cada función.

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `code` | `string` | Identificador estable, como `ERR_ARCHIVE_PATH_ESCAPE`. Consulte los [códigos de error](./index.md#codigos-de-error). |
| `message` | `string` | Descripción legible. |
| `entry` | `string?` | La entrada del archivo o la ruta de origen que causó la falla, cuando la hay, truncada a 4 KiB. |

<!-- @include: ../../../.fragments/modules/archive/types/error.md -->
