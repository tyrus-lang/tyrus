---
title: rm
description: Removes a file or directory.
---

# rm

Removes a file or directory. For directories, use `{ recursive: true }` to remove all contents.

<SignatureBlock label="Signature">rm(path: string, options?: RmOptions): Result&lt;void, FsError&gt;</SignatureBlock>

The operation returns `Err(FsError)` on failure; inspect it with `match` or propagate it with `?`.

## Options

`RmOptions` is an interface with two optional fields:

- `recursive?: boolean`, when `true`, removes directories and all their contents recursively. Defaults to `false`.
- `force?: boolean`, when `true`, silently ignores non-existent paths. Defaults to `false`.

## Example

<!-- @include: ../../.fragments/modules/fs/rm/basic-usage.md -->
