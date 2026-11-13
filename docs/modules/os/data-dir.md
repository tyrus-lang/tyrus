---
title: dataDir
description: Read the user's data directory through the native tyrus:os module.
---

# dataDir

`dataDir` returns the user's platform-standard data directory (e.g. `~/.local/share` on Linux).

<SignatureBlock label="Signature">dataDir(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-data-dir.md -->
::: tip Never fails the program
Falls back to an empty string if it cannot be determined, rather than aborting the program.
:::

See also [`appDataDir`](./app-data-dir.md) for an application-specific subdirectory.
