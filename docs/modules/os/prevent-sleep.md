---
title: preventSleep
description: Inhibit automatic system/display sleep through the native tyrus:os module.
---

# preventSleep

`preventSleep` inhibits automatic system/display sleep until [`allowSleep`](./allow-sleep.md) is called.

<SignatureBlock label="Signature">preventSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-prevent-sleep.md -->
Idempotent: calling this again while sleep is already inhibited is a no-op success.

Implemented per platform:

- **Linux**, spawns `systemd-inhibit --mode=block sleep infinity` and holds the child process alive.
- **macOS**, spawns `caffeinate -dims`.
- **Windows**, calls `SetThreadExecutionState` with `ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED`; no child process involved.

Returns `Err(OsError)` (`code: "ERR_OS_SLEEP_INHIBIT_FAILED"`) if the platform's mechanism is unavailable, e.g. `systemd-inhibit` missing on a non-systemd Linux distribution.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable identifier (e.g. `"ERR_OS_SLEEP_INHIBIT_FAILED"`). |
| `message` | `string` | A human-readable description of what went wrong. |
