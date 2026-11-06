---
title: writeFileStream
description: Escribe chunks binarios limitados progresivamente en un archivo.
---

# writeFileStream

`writeFileStream` crea o trunca un archivo y devuelve un `Result<WritableStream<u8[]>, FsError>` limitado. El backpressure impide que los productores mantengan toda la salida en memoria.

## Firma

<!-- @include: ../../../.fragments/modules/fs/write-file-stream/signature.md -->

La cola contiene como máximo 256 KiB. `await stream.close()` y `await source.pipeTo(stream)` se resuelven solo después de que todos los chunks aceptados se hayan escrito y el buffer del archivo se haya descargado.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/write-file-stream/basic-usage.md -->

Los fallos al abrir devuelven `Err(FsError)`. Los fallos de escritura o flush rechazan las operaciones del stream con `StreamError`.
