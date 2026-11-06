---
title: cp
description: Copies a file or directory.
---

# cp

Copies a file or directory from `src` to `dest`. For directories, use `{ recursive: true }`.

<SignatureBlock label="Signature">cp(src: string, dest: string, options?: CpOptions): Result&lt;void, FsError&gt;</SignatureBlock>

The operation returns `Err(FsError)` on failure; inspect it with `match` or propagate it with `?`.

## Options

`CpOptions` is an interface with two optional fields:

- `recursive?: boolean`, when `true`, copies directories recursively. Required for directory sources. Defaults to `false`.
- `force?: boolean`, when `true`, overwrites existing files at the destination. Defaults to `false`.

## Example

<!-- @include: ../../.fragments/modules/fs/cp/basic-usage.md -->
