---
title: isScreenLocked
description: Detect whether the screensaver/lock screen is active through the native tyrus:os module.
---

# isScreenLocked

`isScreenLocked` returns whether the screensaver/lock screen is currently active.

<SignatureBlock label="Signature">isScreenLocked(): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-is-screen-locked.md -->
::: tip Best effort, Linux-only for now
Queries the freedesktop screensaver D-Bus interface (`org.freedesktop.ScreenSaver.GetActive`, implemented by GNOME, KDE, and most other Linux screensavers/lockers). Returns `false` on any other platform, or when the D-Bus screensaver service cannot be reached, a headless machine, no session bus.
:::

See [`isSessionLocked`](./is-session-locked.md) for the related, but distinct, session-level lock state.
