---
title: diff
description: Compares two files or directories.
---

# diff

Compares two files or directories and returns a `DiffResult` describing the differences.

<SignatureBlock label="Signature">diff(path1: string, path2: string): Result&lt;DiffResult, FsError&gt;</SignatureBlock>

## DiffResult fields

| Field | Type | Description |
|-------|------|-------------|
| `added` | `string[]` | Files present in path1 but not path2. |
| `removed` | `string[]` | Files present in path2 but not path1. |
| `modified` | `string[]` | Files present in both but with different contents. |
| `unchanged` | `string[]` | Files present in both with identical contents. |

## Error handling

Returns `Err(FsError)` with code `ERR_FS_DIFF_FAILED` if either path cannot be inspected, a directory cannot be listed, or a matching file cannot be read. The operation does not report failed reads as differences.

## Example

<!-- @include: ../../.fragments/modules/fs/diff/example.md -->
