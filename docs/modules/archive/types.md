---
title: tyrus:archive types
description: Entry, summary, option, and error types exported by tyrus:archive.
---

# tyrus:archive types

::: tip Types must be imported by name
`ArchiveEntry`, `ArchiveEntryKind`, `ArchiveError`, `ArchiveFormat`, `ArchiveSummary`, `PackOptions`, and `UnpackOptions` must be imported explicitly whenever their fields, members, or object literals are used.
:::

## ArchiveSummary

`ArchiveSummary` is returned by [`pack`](./pack.md) and [`unpack`](./unpack.md).

| Field | Type | Description |
|-------|------|-------------|
| `entries` | `u64` | Files and directories written. |
| `bytes` | `u64` | Uncompressed file bytes written. |

## ArchiveEntry

`ArchiveEntry` is returned by [`list`](./list.md).

| Field | Type | Description |
|-------|------|-------------|
| `name` | `string` | The entry name exactly as the archive stores it. Directory names end with `/` in ZIP archives. |
| `kind` | `ArchiveEntryKind` | What the entry is. |
| `size` | `u64` | Uncompressed size in bytes. |
| `compressedSize` | `u64?` | Compressed size in bytes for ZIP entries; absent for TAR entries. |

## ArchiveEntryKind

| Member | Meaning |
|--------|---------|
| `File` | A regular file. |
| `Directory` | A directory. |
| `Symlink` | A symbolic link. `unpack` refuses it. |
| `HardLink` | A hard link. `unpack` refuses it. |
| `Other` | A device, pipe, or other special entry. `unpack` refuses it. |

<!-- @include: ../../.fragments/modules/archive/types/entry-kind.md -->

## ArchiveFormat

The format [`packStream`](./pack-stream.md) writes, since a stream has no extension to decide it.

| Member | Format |
|--------|--------|
| `Zip` | ZIP, with data descriptors. |
| `Tar` | TAR, uncompressed. |
| `TarGz` | TAR compressed with Gzip. |

## PackOptions

| Field | Type | Default |
|-------|------|---------|
| `level` | `i32?` | `6` |
| `overwrite` | `boolean?` | `false` |
| `exclude` | `string[]?` | none |
| `prefix` | `string?` | none |
| `reproducible` | `boolean?` | `false` |
| `timeoutMs` | `u64?` | `600000` |

See [`pack`](./pack.md#every-option) for what each option does.

## UnpackOptions

| Field | Type | Default |
|-------|------|---------|
| `overwrite` | `boolean?` | `false` |
| `stripComponents` | `u32?` | `0` |
| `maxEntries` | `u64?` | `100000` |
| `maxBytes` | `u64?` | `4 GiB` |
| `maxRatio` | `u32?` | `200` |
| `timeoutMs` | `u64?` | `600000` |

See [`unpack`](./unpack.md#every-option) for what each option does.

## ArchiveError

`ArchiveError` is the recoverable error of every archive operation, delivered as the `Err` of the `Result` each function resolves with.

| Field | Type | Description |
|-------|------|-------------|
| `code` | `string` | Stable identifier, such as `ERR_ARCHIVE_PATH_ESCAPE`. See the [error codes](./index.md#error-codes). |
| `message` | `string` | Human-readable description. |
| `entry` | `string?` | The archive entry or source path that caused the failure, when there is one, truncated to 4 KiB. |

<!-- @include: ../../.fragments/modules/archive/types/error.md -->
