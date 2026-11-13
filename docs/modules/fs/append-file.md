---
title: appendFile
description: Append data to a file asynchronously through the native tyrus:fs module.
---

# appendFile

`appendFile` appends string data to a file. If the file does not exist, it is created.

<SignatureBlock label="Signature">appendFile(path: string, data: string): Promise&lt;Result&lt;void, FsError&gt;&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/fs/append-file/basic-usage.md -->
## Error Handling

If the file cannot be appended to, the fulfilled promise contains `Err(FsError)`. Match the returned result:

::: danger error
<!-- @include: ../../.fragments/modules/fs/append-file/error-handling.md -->
:::
