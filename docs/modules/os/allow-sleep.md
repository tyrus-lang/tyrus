---
title: allowSleep
description: Reverse preventSleep through the native tyrus:os module.
---

# allowSleep

`allowSleep` reverses [`preventSleep`](./prevent-sleep.md), letting the system sleep normally again.

<SignatureBlock label="Signature">allowSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-allow-sleep.md -->
Idempotent: calling this with nothing inhibited is a no-op success, matching `preventSleep`'s own idempotence. Returns `Err(OsError)` with code `ERR_OS_SLEEP_ALLOW_FAILED` if the platform API cannot restore normal sleep behavior.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable identifier. |
| `message` | `string` | A human-readable description of what went wrong. |
