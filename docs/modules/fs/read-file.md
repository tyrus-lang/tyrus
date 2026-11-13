---
title: readFile
description: Read UTF-8 files asynchronously through the native tyrus:fs module.
---

# readFile

`readFile` reads a file asynchronously and returns a `Promise<Result<string, FsError>>` with the file's UTF-8 contents.

<SignatureBlock label="Signature">readFile(path: string): Promise&lt;Result&lt;string, FsError&gt;&gt;</SignatureBlock>

<!-- @include: ../../.fragments/modules/fs/read-file/basic-usage.md -->

## Error Handling

If the file cannot be read, the fulfilled promise contains `Err(FsError)`. The error has a stable `code` field (e.g. `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) and a human-readable `message`. Match the returned result:

::: danger error
<!-- @include: ../../.fragments/modules/fs/read-file/error-handling.md -->
:::

## With Explicit Types

<!-- @include: ../../.fragments/modules/fs/read-file/with-explicit-types.md -->
