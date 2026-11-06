---
title: diskUsage
description: Gets disk usage information.
---

# diskUsage

Returns disk usage information for the filesystem containing the given path.
On Unix, `free` is the space available to this process according to `statvfs`; on Windows, it is the space available to the caller according to `GetDiskFreeSpaceExW`. `used` is the filesystem-wide used space, so `used + free` can be less than `total`.

<SignatureBlock label="Signature">diskUsage(path: string): Result&lt;DiskUsage, FsError&gt;</SignatureBlock>

## Fields

| Field | Type | Description |
|-------|------|-------------|
| `total` | `u64` | Total disk space in bytes. |
| `used` | `u64` | Used disk space in bytes. |
| `free` | `u64` | Bytes this process can still write. Space reserved for the superuser is not included, so `used + free` can be less than `total`. |

## Example

<!-- @include: ../../.fragments/modules/fs/disk-usage/example.md -->
