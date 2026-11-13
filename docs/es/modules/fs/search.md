---
title: search
description: Busca archivos que coinciden con un patrón.
---

# search

Busca archivos que coinciden con un patrón glob, opcionalmente de forma recursiva.

<SignatureBlock label="Firma">search(pattern: string, options?: SearchOptions): Result&lt;string[], FsError&gt;</SignatureBlock>

## Opciones

`SearchOptions` es una interfaz con dos campos opcionales:

- `recursive?: boolean`: cuando es `true`, busca en los subdirectorios de forma recursiva. El valor predeterminado es `false`.
- `maxResults?: u32`: número máximo de resultados devueltos. De forma predeterminada, no hay límite.

## Manejo de errores

Devuelve `Err(FsError)` con el código `ERR_FS_SEARCH_FAILED` si no se puede listar un directorio o inspeccionar una entrada. Una búsqueda fallida no devuelve resultados parciales; si no hay coincidencias, el resultado es `Ok([])`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/search/example.md -->
