---
title: tyrus:archive
description: Crea, inspecciona y extrae de forma segura archivos ZIP, TAR y TAR.GZ.
---

# tyrus:archive

`tyrus:archive` crea, inspecciona y extrae archivos ZIP, TAR y TAR.GZ a partir de rutas en disco. Toda operación es asíncrona, transfiere los datos mediante búferes limitados y nunca carga un archivo completo en memoria; `packStream` y `unpackStream` hacen lo mismo con un stream de bytes en lugar del archivo.

La extracción es segura por defecto: ninguna entrada puede escribirse fuera del directorio de destino, los enlaces y archivos especiales se rechazan, los límites de cantidad de entradas, tamaño extraído y razón de compresión detienen las bombas de descompresión, y toda operación desiste después de un límite de tiempo.

## Imports

<SignatureBlock label="Importaciones">import { ArchiveEntry, ArchiveEntryKind, ArchiveError, ArchiveFormat, ArchiveSummary, PackOptions, UnpackOptions, list, pack, packStream, unpack, unpackStream } from "tyrus:archive";</SignatureBlock>

## Funciones

| Elemento | Firma | Propósito |
|------|-----------|---------|
| [`pack`](./pack.md) | `(source: string, destination: string, options?: PackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Crea un archivo a partir de un archivo o directorio. |
| [`list`](./list.md) | `(archive: string, options?: UnpackOptions) => Promise<Result<ArchiveEntry[], ArchiveError>>` | Lee las entradas de un archivo sin extraerlo. |
| [`unpack`](./unpack.md) | `(archive: string, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extrae un archivo en un directorio. |
| [`packStream`](./pack-stream.md) | `(source: string, format: ArchiveFormat, options?: PackOptions) => ReadableStream<u8[]>` | Escribe el archivo de un fichero o directorio en un stream de bytes. |
| [`unpackStream`](./unpack-stream.md) | `(archive: ReadableStream<u8[]>, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extrae un archivo TAR o TAR.GZ de un stream de bytes. |

## Formatos

`pack` decide el formato por la extensión del destino, que debe estar en minúsculas. `list` y `unpack` ignoran el nombre del archivo e identifican el formato por el contenido, así que un `.jar`, `.epub` o `.docx`, que son archivos ZIP, se extrae como cualquier `.zip`.

| Extensión | Formato | Compresión |
|-----------|--------|-------------|
| `.zip` | ZIP | Deflate, niveles `0` a `9` |
| `.tar` | TAR | Ninguna |
| `.tar.gz`, `.tgz` | TAR.GZ | Gzip, niveles `0` a `9` |

## Códigos de error

Toda función resuelve con un `Result` y nunca rechaza: el éxito es `Ok` con el valor, y una falla es `Err` con un [`ArchiveError`](./types.md#archiveerror). Su `code` es estable, su `message` es legible y su `entry` nombra la entrada del archivo que causó la falla, cuando la hay. Dentro de una función que devuelve un `Result` con el mismo error, `await pack(...)?` entrega el valor o devuelve el error.

| Código | Significado |
|------|---------|
| `ERR_ARCHIVE_NOT_FOUND` | El archivo, el origen o el directorio padre del destino no existe. |
| `ERR_ARCHIVE_IO` | La lectura o la escritura falló. |
| `ERR_ARCHIVE_PERMISSION_DENIED` | El sistema operativo denegó el acceso. |
| `ERR_ARCHIVE_UNSUPPORTED_FORMAT` | El contenido no es un archivo ZIP, TAR o TAR.GZ. |
| `ERR_ARCHIVE_UNSUPPORTED_EXTENSION` | El destino de `pack` no termina en `.zip`, `.tar`, `.tar.gz` o `.tgz`. |
| `ERR_ARCHIVE_INVALID_ARCHIVE` | El archivo está mal formado o truncado. |
| `ERR_ARCHIVE_CHECKSUM_MISMATCH` | Los datos de una entrada no coinciden con su checksum. |
| `ERR_ARCHIVE_ENCRYPTED` | El archivo contiene una entrada cifrada. |
| `ERR_ARCHIVE_INVALID_OPTION` | Una opción tiene un valor inválido, como un límite `0`. |
| `ERR_ARCHIVE_INVALID_COMPRESSION_LEVEL` | `level` está fuera de `0` a `9` o se indicó para un destino `.tar`. |
| `ERR_ARCHIVE_DESTINATION_EXISTS` | El destino ya existe y `overwrite` no está activado. |
| `ERR_ARCHIVE_PATH_ESCAPE` | Una entrada se escribiría fuera del directorio de destino. |
| `ERR_ARCHIVE_INVALID_ENTRY_NAME` | El nombre de una entrada está mal formado o no puede crearse en todas las plataformas. |
| `ERR_ARCHIVE_DUPLICATE_ENTRY` | Dos entradas resultan en la misma ruta. |
| `ERR_ARCHIVE_UNSAFE_LINK` | El archivo contiene un enlace simbólico o duro. |
| `ERR_ARCHIVE_UNSUPPORTED_ENTRY` | El archivo usa un tipo de entrada o método de compresión no soportado, o el origen contiene un enlace o archivo especial. |
| `ERR_ARCHIVE_ENTRY_LIMIT` | El archivo tiene más entradas que `maxEntries`. |
| `ERR_ARCHIVE_SIZE_LIMIT` | La extracción escribiría más bytes que `maxBytes`. |
| `ERR_ARCHIVE_RATIO_LIMIT` | Una entrada, o un archivo TAR.GZ en conjunto, se expande más de `maxRatio` veces su tamaño comprimido. |
| `ERR_ARCHIVE_ALLOCATION_FAILED` | No se pudo reservar memoria para el resultado. |
| `ERR_ARCHIVE_NO_SPACE` | El disco se quedó sin espacio, o un archivo ZIP declara más contenido del que el destino tiene libre. |
| `ERR_ARCHIVE_TIMEOUT` | La operación tardó más que `timeoutMs`. |
| `ERR_ARCHIVE_ZIP_NOT_STREAMABLE` | Se pasó un archivo ZIP a `unpackStream`, que no extrae ZIP de un stream. |

## Ejemplo

<!-- @include: ../../../.fragments/modules/archive/index/example.md -->
