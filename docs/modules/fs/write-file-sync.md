---
title: writeFileSync
description: Write UTF-8 files synchronously through the native tyrus:fs module.
---

# writeFileSync

`writeFileSync` writes string data to a file. If the file does not exist, it is created; if it exists, it is overwritten.

<SignatureBlock label="Signature">writeFileSync(path: string, data: string): Result&lt;void, FsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/fs/write-file-sync/basic-usage.md -->
## Error Handling

If the file cannot be written, `writeFileSync` returns `Err(FsError)` with a stable `code` field. Inspect the result with `match` or propagate it with `?`:

::: danger error
<!-- @include: ../../.fragments/modules/fs/write-file-sync/error-handling.md -->
:::

## Copy a File

<!-- @include: ../../.fragments/modules/fs/write-file-sync/copy.md -->
