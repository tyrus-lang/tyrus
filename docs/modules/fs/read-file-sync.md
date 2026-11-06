---
title: readFileSync
description: Read UTF-8 files synchronously through the native tyrus:fs module.
---

# readFileSync

`readFileSync` returns a file's UTF-8 contents as `Ok(string)`, or an `FsError` as `Err`.

<SignatureBlock label="Signature">readFileSync(path: string): Result&lt;string, FsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/fs/read-file-sync/basic-usage.md -->
## Error Handling

If the file cannot be read, `readFileSync` returns `Err(FsError)` with a stable `code` field (e.g. `ERR_FS_READ_FAILED`, `ERR_FS_NOT_FOUND`) and a human-readable `message`. Inspect the result with `match` or propagate it with `?`:

::: danger error
<!-- @include: ../../.fragments/modules/fs/read-file-sync/error-handling.md -->
:::

## With Explicit Types

<!-- @include: ../../.fragments/modules/fs/read-file-sync/with-explicit-types.md -->
