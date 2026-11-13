---
title: tyrus:archive
description: Create, inspect, and safely extract ZIP, TAR, and TAR.GZ archives.
---

# tyrus:archive

`tyrus:archive` creates, inspects, and extracts ZIP, TAR, and TAR.GZ archives from paths on disk. Every operation is asynchronous, streams data through bounded buffers, and never loads a whole file into memory; `packStream` and `unpackStream` do the same with a byte stream in place of the archive file.

Extraction is safe by default: an entry can never be written outside the destination directory, links and special files are refused, limits on entry count, extracted size, and compression ratio stop decompression bombs, and every operation gives up after a time limit.

## Imports

<SignatureBlock label="Imports">import { ArchiveEntry, ArchiveEntryKind, ArchiveError, ArchiveFormat, ArchiveSummary, PackOptions, UnpackOptions, list, pack, packStream, unpack, unpackStream } from "tyrus:archive";</SignatureBlock>

## Functions

| Item | Signature | Purpose |
|------|-----------|---------|
| [`pack`](./pack.md) | `(source: string, destination: string, options?: PackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Creates an archive from a file or directory. |
| [`list`](./list.md) | `(archive: string, options?: UnpackOptions) => Promise<Result<ArchiveEntry[], ArchiveError>>` | Reads the entries of an archive without extracting it. |
| [`unpack`](./unpack.md) | `(archive: string, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extracts an archive into a directory. |
| [`packStream`](./pack-stream.md) | `(source: string, format: ArchiveFormat, options?: PackOptions) => ReadableStream<u8[]>` | Writes the archive of a file or directory into a byte stream. |
| [`unpackStream`](./unpack-stream.md) | `(archive: ReadableStream<u8[]>, destination: string, options?: UnpackOptions) => Promise<Result<ArchiveSummary, ArchiveError>>` | Extracts a TAR or TAR.GZ archive from a byte stream. |

## Formats

`pack` decides the format from the destination's extension, which must be lowercase. `list` and `unpack` ignore the file name and identify the format from the content, so a `.jar`, `.epub`, or `.docx` file, which are ZIP archives, extracts like any `.zip`.

| Extension | Format | Compression |
|-----------|--------|-------------|
| `.zip` | ZIP | Deflate, levels `0` to `9` |
| `.tar` | TAR | None |
| `.tar.gz`, `.tgz` | TAR.GZ | Gzip, levels `0` to `9` |

## Error codes

Every function resolves with a `Result` and never rejects: success is `Ok` with the value, and a failure is `Err` with an [`ArchiveError`](./types.md#archiveerror). Its `code` is stable, its `message` is readable, and its `entry` names the archive entry that caused the failure when there is one. Inside a function that returns a `Result` with the same error, `await pack(...)?` hands the value over or returns the error.

| Code | Meaning |
|------|---------|
| `ERR_ARCHIVE_NOT_FOUND` | The archive, the source, or the destination's parent directory does not exist. |
| `ERR_ARCHIVE_IO` | Reading or writing failed. |
| `ERR_ARCHIVE_PERMISSION_DENIED` | The operating system denied access. |
| `ERR_ARCHIVE_UNSUPPORTED_FORMAT` | The content is not a ZIP, TAR, or TAR.GZ archive. |
| `ERR_ARCHIVE_UNSUPPORTED_EXTENSION` | The destination of `pack` does not end with `.zip`, `.tar`, `.tar.gz`, or `.tgz`. |
| `ERR_ARCHIVE_INVALID_ARCHIVE` | The archive is malformed or truncated. |
| `ERR_ARCHIVE_CHECKSUM_MISMATCH` | An entry's data does not match its checksum. |
| `ERR_ARCHIVE_ENCRYPTED` | The archive contains an encrypted entry. |
| `ERR_ARCHIVE_INVALID_OPTION` | An option has an invalid value, such as a limit of `0`. |
| `ERR_ARCHIVE_INVALID_COMPRESSION_LEVEL` | `level` is outside `0` to `9`, or was given for a `.tar` destination. |
| `ERR_ARCHIVE_DESTINATION_EXISTS` | The destination already exists and `overwrite` is not set. |
| `ERR_ARCHIVE_PATH_ESCAPE` | An entry would be written outside the destination directory. |
| `ERR_ARCHIVE_INVALID_ENTRY_NAME` | An entry name is malformed or cannot be created on every platform. |
| `ERR_ARCHIVE_DUPLICATE_ENTRY` | Two entries resolve to the same path. |
| `ERR_ARCHIVE_UNSAFE_LINK` | The archive contains a symbolic or hard link. |
| `ERR_ARCHIVE_UNSUPPORTED_ENTRY` | The archive uses an unsupported entry type or compression method, or the source contains a link or special file. |
| `ERR_ARCHIVE_ENTRY_LIMIT` | The archive has more entries than `maxEntries`. |
| `ERR_ARCHIVE_SIZE_LIMIT` | Extraction would write more bytes than `maxBytes`. |
| `ERR_ARCHIVE_RATIO_LIMIT` | An entry, or a TAR.GZ archive as a whole, expands more than `maxRatio` times its compressed size. |
| `ERR_ARCHIVE_ALLOCATION_FAILED` | Memory could not be allocated for the result. |
| `ERR_ARCHIVE_NO_SPACE` | The disk ran out of space, or a ZIP archive declares more content than the destination has free. |
| `ERR_ARCHIVE_TIMEOUT` | The operation took longer than `timeoutMs`. |
| `ERR_ARCHIVE_ZIP_NOT_STREAMABLE` | A ZIP archive was given to `unpackStream`, which cannot extract ZIP from a stream. |

## Example

<!-- @include: ../../.fragments/modules/archive/index/example.md -->
