---
title: unpackStream
description: Extrae un archivo TAR o TAR.GZ de un stream de bytes a medida que llega.
---

# unpackStream

`unpackStream` extrae un archivo TAR o TAR.GZ de un `ReadableStream<u8[]>` a un directorio mientras llegan los bytes, con la misma seguridad que [`unpack`](./unpack.md): las entradas quedan dentro del destino, los enlaces y ficheros especiales se rechazan, los límites se aplican y, ante un fallo, todo lo que creó la llamada se elimina. El formato se identifica por los primeros bytes.

Un archivo ZIP no se puede extraer de un stream: su tabla de entradas está al final, y las entradas anteriores no son confiables hasta leerla. Un stream así falla con `ERR_ARCHIVE_ZIP_NOT_STREAMABLE`; escríbelo en un fichero y usa `unpack`.

<SignatureBlock label="Firma">unpackStream(archive: ReadableStream&lt;u8[]&gt;, destination: string, options?: UnpackOptions): Promise&lt;Result&lt;ArchiveSummary, ArchiveError&gt;&gt;</SignatureBlock>

## Ejemplo mínimo

<!-- @include: ../../../.fragments/modules/archive/unpack-stream/minimal.md -->

## Todas las opciones

<!-- @include: ../../../.fragments/modules/archive/unpack-stream/maximal.md -->

El stream se toma prestado: la llamada lo lee hasta el final del archivo, y el programa conserva su handle. Un stream que falla durante la lectura se resuelve con `ERR_ARCHIVE_IO`, y uno en el que nadie escribe falla en `timeoutMs`.
