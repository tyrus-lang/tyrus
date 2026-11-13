---
title: trash
description: Move a path to the platform trash or recycle bin through the native tyrus:os module.
---

# trash

`trash` moves `path` to the platform trash/recycle bin, recoverable, unlike deleting it outright.

<SignatureBlock label="Signature">trash(path: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-trash.md -->
Returns `Err(OsError)` (`code: "ERR_OS_TRASH_FAILED"`) if `path` does not exist or cannot be moved to the trash.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable identifier (e.g. `"ERR_OS_TRASH_FAILED"`). |
| `message` | `string` | A human-readable description of what went wrong. |
