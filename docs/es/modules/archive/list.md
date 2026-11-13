---
title: list
description: Lee las entradas de un archivo sin extraerlo.
---

# list

`list` lee las entradas de un archivo ZIP, TAR o TAR.GZ sin escribir nada en disco. El formato se identifica por el contenido, nunca por el nombre del archivo.

Úselo para inspeccionar un archivo no confiable antes de extraerlo: las entradas que `unpack` rechazaría, como los enlaces, aparecen con su tipo, y los nombres se informan exactamente como el archivo los guarda.

<SignatureBlock label="Firma">list(archive: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveEntry[], ArchiveError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/list/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/archive/list/maximal.md -->

`list` solo lee `maxEntries` y `timeoutMs` de [`UnpackOptions`](./types.md#unpackoptions); las demás opciones se aplican a la extracción. Un archivo con más entradas falla con `ERR_ARCHIVE_ENTRY_LIMIT`, y un archivo ZIP falla antes de leer su tabla de entradas.
