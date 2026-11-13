---
title: isDarkMode
description: Detect whether the desktop is in dark mode through the native tyrus:os module.
---

# isDarkMode

`isDarkMode` returns whether the desktop is currently in dark mode.

<SignatureBlock label="Signature">isDarkMode(): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-is-dark-mode.md -->
::: tip Best effort
Relies on the platform having a queryable dark-mode setting (the Windows registry, macOS's `AppleInterfaceStyle`, or a Linux desktop portal/GNOME setting). Where none is reachable, a headless machine, an unsupported desktop, this conservatively returns `false` (light) rather than aborting the program.
:::
