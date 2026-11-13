---
title: diskUsage
description: Obtiene información sobre el uso del disco.
---

# diskUsage

Devuelve información sobre el uso del disco del sistema de archivos que contiene la ruta indicada.
En Unix, `free` es el espacio disponible para este proceso según `statvfs`; en Windows, es el espacio disponible para el llamador según `GetDiskFreeSpaceExW`. `used` representa el espacio usado en todo el sistema de archivos, por lo que `used + free` puede ser menor que `total`.

<SignatureBlock label="Firma">diskUsage(path: string): Result&lt;DiskUsage, FsError&gt;</SignatureBlock>

## Campos

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `total` | `u64` | Espacio total del disco en bytes. |
| `used` | `u64` | Espacio usado del disco en bytes. |
| `free` | `u64` | Bytes que este proceso todavía puede escribir. El espacio reservado al superusuario no se incluye, así que `used + free` puede ser menor que `total`. |

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/disk-usage/example.md -->
