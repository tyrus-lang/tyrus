---
title: tempDir
description: Devuelve el directorio temporal del sistema.
---

# tempDir

Devuelve una struct `TempDir` con un campo `path` que contiene la ruta del directorio temporal del sistema.

<SignatureBlock label="Firma">tempDir(): TempDir</SignatureBlock>

## Campos de TempDir

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `path` | `string` | Ruta al directorio temporal del sistema. |

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/temp-dir/example.md -->
