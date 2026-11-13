---
title: tempDir
description: Returns the system temp directory.
---

# tempDir

Returns a `TempDir` struct with a `path` field containing the system's temporary directory path.

<SignatureBlock label="Signature">tempDir(): TempDir</SignatureBlock>

## TempDir fields

| Field | Type | Description |
|-------|------|-------------|
| `path` | `string` | The path to the system temp directory. |

## Example

<!-- @include: ../../.fragments/modules/fs/temp-dir/example.md -->
