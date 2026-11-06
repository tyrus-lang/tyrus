---
title: openUrl
description: Open a URL with the platform's default handler through the native tyrus:os module.
---

# openUrl

`openUrl` opens `url` with the platform's default handler (browser, mail client, etc.), the same effect as double-clicking a link.

<SignatureBlock label="Signature">openUrl(url: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-open-url.md -->
Returns `Err(OsError)` (`code: "ERR_OS_NO_HANDLER"`) if no application is registered to open `url`.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable identifier (e.g. `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | A human-readable description of what went wrong. |
