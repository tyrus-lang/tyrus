---
title: tyrus:fs
description: APIs nativas de filesystem con soporte async, respaldadas por el runtime de tyrus.
---

# tyrus:fs

`tyrus:fs` expone operaciones nativas de filesystem proporcionadas por el runtime nativo. Proporciona variantes síncronas y asíncronas con `FsError` recuperable.

## Importaciones

<!-- @include: ../../../.fragments/modules/fs/imports.md -->

## Funciones

| Función | Firma | Finalidad |
|---------|-------|-----------|
| [`readFile`](./read-file.md) | `(path: string) => Promise<Result<string, FsError>>` | Lee un archivo UTF-8 de forma asíncrona. |
| [`readFileStream`](./read-file-stream.md) | `(path: string) => Result<ReadableStream<u8[]>, FsError>` | Lee chunks binarios limitados progresivamente. |
| [`readFileSync`](./read-file-sync.md) | `(path: string) => Result<string, FsError>` | Lee un archivo UTF-8 de forma síncrona. |
| [`writeFile`](./write-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Escribe una string en un archivo de forma asíncrona. |
| [`writeFileStream`](./write-file-stream.md) | `(path: string) => Result<WritableStream<u8[]>, FsError>` | Escribe chunks binarios limitados progresivamente. |
| [`writeFileSync`](./write-file-sync.md) | `(path: string, data: string) => Result<void, FsError>` | Escribe una string en un archivo de forma síncrona. |
| [`exists`](./exists.md) | `(path: string) => boolean` | Verifica si una ruta existe. |
| [`appendFile`](./append-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Añade datos a un archivo de forma asíncrona. |
| [`readdir`](./readdir.md) | `(path: string, options?: ReaddirOptions) => Result<DirEntry[], FsError>` | Lee entradas de un directorio. |
| [`mkdir`](./mkdir.md) | `(path: string) => Result<void, FsError>` | Crea un directorio. |
| [`rm`](./rm.md) | (path: string, options?: RmOptions) => Result<void, FsError> | Elimina un archivo o directorio. |
| [`cp`](./cp.md) | (src: string, dest: string, options?: CpOptions) => Result<void, FsError> | Copia un archivo o directorio. |
| [`mv`](./mv.md) | (src: string, dest: string) => Result<void, FsError> | Mueve un archivo o directorio. |
| [`removeAll`](./remove-all.md) | (path: string) => Result<void, FsError> | Elimina recursivamente un directorio. |
| [`stat`](./stat.md) | `(path: string) => Result<FileStats, FsError>` | Obtiene estadísticas del archivo o un error estructurado del sistema de archivos. |
| [`metadata`](./metadata.md) | `(path: string) => Result<Metadata, FsError>` | Obtiene metadatos detallados. |
| [`diskUsage`](./disk-usage.md) | `(path: string) => Result<DiskUsage, FsError>` | Obtiene información de uso de disco. |
| [`mime`](./mime.md) | `(path: string) => string?` | Obtiene el tipo MIME de la extensión. |
| [`chmod`](./chmod.md) | `(path: string, mode: FileMode) => Result<void, FsError>` | Cambia permisos del archivo. |
| [`chown`](./chown.md) | `(path: string, uid: u32, gid: u32) => Result<void, FsError>` | Cambia la propiedad del archivo. |
| [`readlink`](./readlink.md) | `(path: string) => Result<string, FsError>` | Lee el objetivo de un enlace simbólico. |
| [`symlink`](./symlink.md) | `(original: string, link: string) => Result<void, FsError>` | Crea un enlace simbólico. |
| [`symlinkDir`](./symlink-dir.md) | `(original: string, link: string) => Result<void, FsError>` | Crea un enlace simbólico a un directorio. |
| [`hardLink`](./hard-link.md) | `(original: string, link: string) => Result<void, FsError>` | Crea un enlace duro. |
| [`canonicalize`](./canonicalize.md) | `(path: string) => Result<string, FsError>` | Resuelve a una ruta absoluta. |
| [`glob`](./glob.md) | `(pattern: string) => Result<string[], FsError>` | Encuentra archivos por patrón glob. |
| [`search`](./search.md) | `(pattern: string, options?: SearchOptions) => Result<string[], FsError>` | Busca archivos por patrón. |
| [`diff`](./diff.md) | `(path1: string, path2: string) => Result<DiffResult, FsError>` | Compara dos archivos o directorios. |
| [`atomicWrite`](./atomic-write.md) | `(path: string, data: string) => Result<void, FsError>` | Escribe de forma atómica. |
| [`tempDir`](./temp-dir.md) | `() => TempDir` | Devuelve el directorio temporal. |
| [`tempFile`](./temp-file.md) | `() => Result<string, FsError>` | Crea un archivo temporal. |
| [`watch`](./watch.md) | `(path: string) => Result<FSWatcher, FsError>` | Comienza a observar un archivo o directorio. |

::: warning Errores recuperables
Todas las operaciones recuperables del sistema de archivos devuelven `Result<T, FsError>`. Las llamadas síncronas devuelven `Result<T, FsError>` directamente, mientras que las operaciones asíncronas de archivos devuelven `Promise<Result<T, FsError>>`; los constructores de streams devuelven `Result<ReadableStream<u8[]>, FsError>` o `Result<WritableStream<u8[]>, FsError>`. Usa `match` para tratar el resultado o propágalo con `?`. El error tiene un campo `code` estable (p. ej., `ERR_FS_NOT_FOUND`) y un `message` legible.
:::

## Rutas multiplataforma

Toda ruta pasada a cualquier función `fs` se resuelve de forma transparente antes de llegar al sistema operativo, sin exigir ningún import de módulo `path`:

- **Separadores**: una barra invertida escrita al estilo Windows (`"data\\file.txt"`) se normaliza a barra normal en Linux/macOS. El propio Windows ya acepta nativamente ambas formas, así que ahí no cambia nada.
- **Segmentos relativos**: `./` y `../` se resuelven contra el directorio de trabajo actual, `sub/../file.txt` y `file.txt` apuntan al mismo lugar, y ninguno de los dos exige que `sub` exista de verdad en el disco.

Esta es una resolución **léxica**, una resolución léxica de rutas, no una canonicalización de sistema de archivos: nunca toca el sistema de archivos y nunca exige que la ruta ya exista. Exigir existencia rompería toda llamada que crea un archivo nuevo, que es el caso común de `writeFileSync`.

## Ejemplo

<!-- @include: ../../../.fragments/modules/fs/example.md -->
