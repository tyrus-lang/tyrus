---
title: clipboardWrite
description: Write text to the system clipboard through the native tyrus:os module.
---

# clipboardWrite

`clipboardWrite` requests writing `text` to the system clipboard, replacing whatever was there, and returns a structured error if the clipboard cannot accept it.

<SignatureBlock label="Signature">clipboardWrite(text: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-clipboard-write.md -->
Returns `ERR_OS_INVALID_ARGUMENT` for a null string, `ERR_OS_CLIPBOARD_UNAVAILABLE` if the system clipboard cannot be opened, or `ERR_OS_CLIPBOARD_WRITE_FAILED` if the clipboard rejects the text. Handle the returned `Result` before assuming the clipboard was updated.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable error identifier. |
| `message` | `string` | A human-readable description of the failure. |

::: warning X11: the write is not necessarily durable yet when this call returns
On Linux under X11 (unlike Wayland, macOS, or Windows), the clipboard is not centrally stored by the OS, the writing program itself keeps serving the content on request, handing that duty off to a short-lived background process once it exits. Reading it back immediately afterward, in the same program, can transiently see it as absent before that handoff finishes. A real paste (a human pressing a key a moment later) is never fast enough to hit this; only an automated write-then-read in the same instant is.
:::
