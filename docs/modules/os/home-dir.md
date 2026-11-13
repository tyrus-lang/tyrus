---
title: homeDir
description: Read the user's home directory through the native tyrus:os module.
---

# homeDir

`homeDir` returns the user's home directory.

<SignatureBlock label="Signature">homeDir(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-home-dir.md -->
::: tip Never fails the program
Falls back to an empty string if it cannot be determined, rather than aborting the program.
:::
