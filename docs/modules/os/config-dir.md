---
title: configDir
description: Read the user's config directory through the native tyrus:os module.
---

# configDir

`configDir` returns the user's platform-standard config directory (e.g. `~/.config` on Linux).

<SignatureBlock label="Signature">configDir(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-config-dir.md -->
::: tip Never fails the program
Falls back to an empty string if it cannot be determined, rather than aborting the program.
:::
