---
title: isSessionLocked
description: Detect whether the current user session is locked through the native tyrus:os module.
---

# isSessionLocked

`isSessionLocked` returns whether the current user session is locked.

<SignatureBlock label="Signature">isSessionLocked(): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-is-session-locked.md -->
::: tip Best effort, Linux-only for now
Queries `systemd-logind`'s per-session `LockedHint` property via `loginctl`, the session-level lock flag tracked by every desktop that registers a session with `logind` (GNOME, KDE, and most others on Linux). Returns `false` on any other platform, or when the session state cannot be reached.
:::

See [`isScreenLocked`](./is-screen-locked.md) for the related, but distinct, screensaver state.
