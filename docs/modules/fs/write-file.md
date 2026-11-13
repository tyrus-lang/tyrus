---
title: writeFile
description: Write UTF-8 files asynchronously through the native tyrus:fs module.
---

# writeFile

`writeFile` writes string data to a file asynchronously. If the file does not exist, it is created; if it exists, it is overwritten.

<SignatureBlock label="Signature">writeFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/fs/write-file/basic-usage.md -->
## Error Handling

If the file cannot be written, the fulfilled promise contains `Err(FsError)`. Match the returned result:

::: danger error
<!-- @include: ../../.fragments/modules/fs/write-file/error-handling.md -->
:::

## Copy a File

<!-- @include: ../../.fragments/modules/fs/write-file/copy.md -->
