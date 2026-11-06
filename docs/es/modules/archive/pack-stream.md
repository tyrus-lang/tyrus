---
title: packStream
description: Escribe un archivo en un stream de bytes a medida que se lee el origen.
---

# packStream

`packStream` devuelve un `ReadableStream<u8[]>` con el archivo de un fichero o directorio a medida que se escribe, en el formato indicado: nada se escribe en disco, así que el archivo puede ir directo a una respuesta HTTP, a un stream de compresión o a un fichero. El stream guarda como máximo 64 KiB, y el archivo solo se produce a la velocidad en que se lee.

El ZIP se escribe con data descriptors después de cada entrada, ya que un stream no puede volver atrás para rellenar tamaños; toda herramienta actual lo lee.

<SignatureBlock label="Firma">packStream(source: string, format: ArchiveFormat, options?: PackOptions): ReadableStream&lt;u8[]&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/pack-stream/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/archive/pack-stream/maximal.md -->

`overwrite` no tiene efecto, ya que no hay destino. Un fallo, desde un origen inexistente hasta el límite de tiempo, hace que el stream falle con el [`ArchiveError`](./types.md#archiveerror) en lugar de terminarlo, así que un lector nunca confunde un archivo truncado con uno completo: `read()` y `pipeTo()` rechazan, y `catch (error: ArchiveError)` lo recibe.
