---
title: cacheDir
description: Read the user's cache directory through the native tyrus:os module.
---

# cacheDir

`cacheDir` returns the user's platform-standard cache directory (e.g. `~/.cache` on Linux).

<SignatureBlock label="Signature">cacheDir(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-cache-dir.md -->
::: tip Never fails the program
Falls back to an empty string if it cannot be determined, rather than aborting the program.
:::
