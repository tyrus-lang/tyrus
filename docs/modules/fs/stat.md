---
title: stat
description: Gets file statistics.
---

# stat

Returns statistics about a file or directory. Invalid paths return `Err` with `ERR_FS_INVALID_PATH`; paths that cannot be inspected return `Err` with `ERR_FS_STAT_FAILED`.

When `path` is a symbolic link, the statistics describe its target and `isSymlink` is `true` for the original path.

<SignatureBlock label="Signature">stat(path: string): Result&lt;FileStats, FsError&gt;</SignatureBlock>

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `size` | `u64` | File size in bytes. |
| `isFile` | `boolean` | True if the path is a file. |
| `isDirectory` | `boolean` | True if the path is a directory. |
| `isSymlink` | `boolean` | True if the path is a symbolic link. |
| `modified` | `u64` | Last modification time (Unix epoch seconds). |
| `created` | `u64` | Creation time (Unix epoch seconds). |
| `permissions` | `u32` | File permission bits. |

For an accessible path on non-Unix platforms, `permissions` is `0`; this API exposes POSIX permission bits and does not translate Windows ACLs.

## Example

<!-- @include: ../../.fragments/modules/fs/stat/example.md -->
