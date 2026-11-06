---
title: search
description: Searches for files matching a pattern.
---

# search

Searches for files matching a glob pattern, optionally recursively.

<SignatureBlock label="Signature">search(pattern: string, options?: SearchOptions): Result&lt;string[], FsError&gt;</SignatureBlock>

## Options

`SearchOptions` is an interface with two optional fields:

- `recursive?: boolean`, when `true`, searches subdirectories recursively. Defaults to `false`.
- `maxResults?: u32`, maximum number of results to return. Defaults to no limit.

## Error handling

Returns `Err(FsError)` with code `ERR_FS_SEARCH_FAILED` if a directory cannot be listed or an entry cannot be inspected. A failed search does not return partial results; no matches produce `Ok([])`.

## Example

<!-- @include: ../../.fragments/modules/fs/search/example.md -->
