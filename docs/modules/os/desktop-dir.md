---
title: desktopDir
description: Read the user's desktop directory through the native tyrus:os module.
---

# desktopDir

`desktopDir` returns the user's desktop directory.

<SignatureBlock label="Signature">desktopDir(): string?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-desktop-dir.md -->
::: tip Absence is a valid outcome
A headless system (a server, a container, CI) commonly has no desktop directory at all, that is a normal result, not a failure.
:::
