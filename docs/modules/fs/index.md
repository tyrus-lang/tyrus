---
title: tyrus:fs
description: Native filesystem APIs backed by the tyrus runtime.
---

# tyrus:fs

`tyrus:fs` exposes native filesystem operations provided by the native runtime. It provides both synchronous and asynchronous variants with recoverable `FsError`.

## Imports

<!-- @include: ../../.fragments/modules/fs/imports.md -->

## Functions

| Function | Signature | Purpose |
|----------|-----------|---------|
| [`readFile`](./read-file.md) | `(path: string) => Promise<Result<string, FsError>>` | Reads a UTF-8 file asynchronously. |
| [`readFileStream`](./read-file-stream.md) | `(path: string) => Result<ReadableStream<u8[]>, FsError>` | Reads bounded binary chunks progressively. |
| [`readFileSync`](./read-file-sync.md) | `(path: string) => Result<string, FsError>` | Reads a UTF-8 file synchronously. |
| [`writeFile`](./write-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Writes a string to a file asynchronously. |
| [`writeFileStream`](./write-file-stream.md) | `(path: string) => Result<WritableStream<u8[]>, FsError>` | Writes bounded binary chunks progressively. |
| [`writeFileSync`](./write-file-sync.md) | `(path: string, data: string) => Result<void, FsError>` | Writes a string to a file synchronously. |
| [`exists`](./exists.md) | `(path: string) => boolean` | Checks whether a path exists. |
| [`appendFile`](./append-file.md) | `(path: string, data: string) => Promise<Result<void, FsError>>` | Appends data to a file asynchronously. |
| [`readdir`](./readdir.md) | `(path: string, options?: ReaddirOptions) => Result<DirEntry[], FsError>` | Reads directory entries. |
| [`mkdir`](./mkdir.md) | `(path: string) => Result<void, FsError>` | Creates a directory. |
| [`rm`](./rm.md) | (path: string, options?: RmOptions) => Result<void, FsError> | Removes a file or directory. |
| [`cp`](./cp.md) | (src: string, dest: string, options?: CpOptions) => Result<void, FsError> | Copies a file or directory. |
| [`mv`](./mv.md) | (src: string, dest: string) => Result<void, FsError> | Moves a file or directory. |
| [`removeAll`](./remove-all.md) | (path: string) => Result<void, FsError> | Recursively removes a directory. |
| [`stat`](./stat.md) | `(path: string) => Result<FileStats, FsError>` | Gets file statistics or a structured filesystem error. |
| [`metadata`](./metadata.md) | `(path: string) => Result<Metadata, FsError>` | Gets detailed file metadata. |
| [`diskUsage`](./disk-usage.md) | `(path: string) => Result<DiskUsage, FsError>` | Gets disk usage information. |
| [`mime`](./mime.md) | `(path: string) => string?` | Gets MIME type from file extension. |
| [`chmod`](./chmod.md) | `(path: string, mode: FileMode) => Result<void, FsError>` | Changes file permissions. |
| [`chown`](./chown.md) | `(path: string, uid: u32, gid: u32) => Result<void, FsError>` | Changes file ownership. |
| [`readlink`](./readlink.md) | `(path: string) => Result<string, FsError>` | Reads a symlink target or returns a structured filesystem error. |
| [`symlink`](./symlink.md) | `(original: string, link: string) => Result<void, FsError>` | Creates a symlink. |
| [`symlinkDir`](./symlink-dir.md) | `(original: string, link: string) => Result<void, FsError>` | Creates a directory symlink. |
| [`hardLink`](./hard-link.md) | `(original: string, link: string) => Result<void, FsError>` | Creates a hard link. |
| [`canonicalize`](./canonicalize.md) | `(path: string) => Result<string, FsError>` | Resolves to an absolute path. |
| [`glob`](./glob.md) | `(pattern: string) => Result<string[], FsError>` | Finds files matching a glob pattern. |
| [`search`](./search.md) | `(pattern: string, options?: SearchOptions) => Result<string[], FsError>` | Searches for files matching a pattern. |
| [`diff`](./diff.md) | `(path1: string, path2: string) => Result<DiffResult, FsError>` | Compares two files or directories. |
| [`atomicWrite`](./atomic-write.md) | `(path: string, data: string) => Result<void, FsError>` | Writes atomically (temp + rename). |
| [`tempDir`](./temp-dir.md) | `() => TempDir` | Returns the system temp directory. |
| [`tempFile`](./temp-file.md) | `() => Result<string, FsError>` | Creates a temp file and returns its path. |
| [`watch`](./watch.md) | `(path: string) => Result<FSWatcher, FsError>` | Starts watching a file or directory. |

::: warning Recoverable errors
Every recoverable filesystem operation returns `Result<T, FsError>`. Synchronous calls return `Result<T, FsError>` directly, while asynchronous file operations return `Promise<Result<T, FsError>>`; stream constructors return `Result<ReadableStream<u8[]>, FsError>` or `Result<WritableStream<u8[]>, FsError>`. Match the result or propagate it with `?`. The error has a stable `code` field (e.g. `ERR_FS_NOT_FOUND`) and a human-readable `message`.
:::

## Cross-platform paths

Every path passed to any `fs` function is resolved transparently before it reaches the operating system, with no `path` module import required:

- **Separators**: a backslash written the Windows way (`"data\\file.txt"`) is normalized to a forward slash on Linux/macOS. Windows itself already accepts both natively, so nothing changes there.
- **Relative segments**: `./` and `../` are resolved against the current working directory, `sub/../file.txt` and `file.txt` refer to the same location, and neither requires `sub` to actually exist on disk.

This is a *lexical* resolution, a lexical path resolution, not filesystem canonicalization: it never touches the filesystem and never requires the path to already exist. Requiring existence would break every call that creates a new file, which is the common case for `writeFileSync`.

## Example

<!-- @include: ../../.fragments/modules/fs/example.md -->
