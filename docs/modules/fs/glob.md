---
title: glob
description: Finds files matching a glob pattern.
---

# glob

Matches file and directory names against a glob pattern. The pattern is resolved against the process working directory before matching. Relative patterns return relative paths, while absolute patterns return absolute paths. Supports `*` (matches any characters except `/`) and `**` (matches any number of directories).

<SignatureBlock label="Signature">glob(pattern: string): Result&lt;string[], FsError&gt;</SignatureBlock>

## Error handling

Returns `Err(FsError)` with code `ERR_FS_GLOB_FAILED` if a directory cannot be listed or an entry needed for traversal cannot be inspected. A failed traversal does not return partial matches; no matches produce `Ok([])`.

## Example

<!-- @include: ../../.fragments/modules/fs/glob/example.md -->
