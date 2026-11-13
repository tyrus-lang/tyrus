---
title: metadata
description: Gets detailed file metadata.
---

# metadata

Returns detailed metadata about a file or directory. Metadata fields describe the link target when `path` is a symbolic link, and `isSymlink` indicates that the original path is a link. Returns `Err(FsError)` on failure.

<SignatureBlock label="Signature">metadata(path: string): Result&lt;Metadata, FsError&gt;</SignatureBlock>

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `size` | `u64` | File size in bytes. |
| `isFile` | `boolean` | True if the path is a file. |
| `isDirectory` | `boolean` | True if the path is a directory. |
| `isSymlink` | `boolean` | True if the path is a symbolic link. |
| `modified` | `u64` | Last modification time (Unix epoch seconds). |
| `created` | `u64` | Creation time (Unix epoch seconds). |
| `accessed` | `u64` | Last access time (Unix epoch seconds). |
| `permissions` | `u32` | File permission bits. |
| `uid` | `u32` | User ID of the file owner. |
| `gid` | `u32` | Group ID of the file owner. |
| `mode` | `u32` | File type and permission mode. |

On non-Unix platforms, `permissions`, `uid`, `gid`, and `mode` are currently `0`. These fields expose POSIX permission bits and Unix owner IDs; Windows ACLs are not translated into this representation.

## Error handling

<!-- @include: ../../.fragments/modules/fs/metadata/error-handling.md -->
