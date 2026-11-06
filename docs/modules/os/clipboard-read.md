---
title: clipboardRead
description: Read the system clipboard's current text through the native tyrus:os module.
---

# clipboardRead

`clipboardRead` reads the system clipboard's current text and returns a structured error if the clipboard service is unavailable or cannot be read.

<SignatureBlock label="Signature">clipboardRead(): Result&lt;string?, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-clipboard-read.md -->
Returns `Ok(absent)` if the clipboard is empty or holds non-text content (an image or a file list). It returns `Err` with `ERR_OS_CLIPBOARD_UNAVAILABLE` if no clipboard service is reachable, such as on a headless machine, or `ERR_OS_CLIPBOARD_READ_FAILED` if the text cannot be read. See [`clipboardWrite`](./clipboard-write.md) for a related X11-specific timing caveat when reading back a value this same program just wrote.
