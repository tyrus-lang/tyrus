---
title: hasCapability
description: Check whether a system capability is available through the native tyrus:os module.
---

# hasCapability

`hasCapability` checks whether a `SystemCapability` is available on this system.

<SignatureBlock label="Signature">hasCapability(cap: SystemCapability): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-has-capability.md -->
Answers "can I find out?"/"would this work?" without triggering the real effect, checking `Notification` does not pop up a notification, checking `SleepPrevention` does not spawn an inhibitor process.

### SystemCapability Members

| Member | Meaning |
|--------|---------|
| `Notification` | Whether a desktop notification could plausibly be shown. |
| `Clipboard` | Whether the system clipboard can be reached. |
| `Battery` | Whether this machine reports at least one battery. |
| `DarkMode` | Whether the desktop's dark-mode setting is queryable at all. |
| `ScreenLock` | Whether the screensaver/lock-screen state is queryable at all (Linux only). |
| `SessionLock` | Whether the session-lock state is queryable at all (Linux only). |
| `SleepPrevention` | Whether `preventSleep()`/`allowSleep()` would actually work. |
| `Fonts` | Whether `systemFonts()` would return anything. |

::: tip Not the same question as the function it mirrors
`hasCapability(SystemCapability.ScreenLock)` asks whether the screensaver state is queryable at all, not whether the screen is currently locked, that is [`isScreenLocked()`](./is-screen-locked.md)'s own return value. The same distinction applies to every other capability that mirrors a real function (`Battery` vs. [`battery()`](./battery.md), `DarkMode` vs. [`isDarkMode()`](./is-dark-mode.md), and so on).
:::
