---
title: readFileStream
description: Lee un archivo progresivamente como chunks binarios limitados.
---

# readFileStream

`readFileStream` abre un archivo y devuelve un `Result<ReadableStream<u8[]>, FsError>` limitado. El runtime lee chunks progresivamente, por lo que el uso de memoria no crece con el tamaño completo del archivo.

## Firma

<!-- @include: ../../../.fragments/modules/fs/read-file-stream/signature.md -->

El tamaño predeterminado del chunk es 64 KiB y la cola limitada contiene como máximo 256 KiB. Cada chunk conserva los bytes originales, incluidos bytes cero y datos que no son texto válido.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/read-file-stream/basic-usage.md -->

Los fallos al abrir devuelven `Err(FsError)`. Los fallos posteriores a la apertura del archivo rechazan las operaciones del stream con `StreamError`.
