---
title: notify
description: Show a native desktop notification through the native tyrus:os module.
---

# notify

`notify` requests a native desktop notification and returns a structured error if the platform service cannot show it.

<SignatureBlock label="Signature">notify(title: string, body: string, options?: NotifyOptions): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-notify.md -->
`options` defaults to the platform's own defaults for every field when omitted.

### NotifyOptions Fields

| Field | Type | Meaning |
|-------|------|---------|
| `icon` | `string?` | The name or path of an icon to show alongside the notification. |
| `urgency` | `NotifyUrgency?` | See [NotifyUrgency](#notifyurgency-members). Defaults to `Normal`. |
| `timeout` | `u32?` | How long the notification stays visible, in milliseconds, before the desktop environment dismisses it automatically. |

### NotifyUrgency Members

| Member | Meaning |
|--------|---------|
| `Low` | Low priority, the desktop environment may not show it at all in "do not disturb" mode. |
| `Normal` | Default priority. |
| `Critical` | High priority, some desktop environments keep it on screen until dismissed. |

Returns `Err(OsError)` with code `ERR_OS_NOTIFICATION_FAILED` when the platform notification service is unavailable or rejects the notification. A null title or body returns `ERR_OS_INVALID_ARGUMENT`.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable error identifier. |
| `message` | `string` | A human-readable description of the failure. |
:::
