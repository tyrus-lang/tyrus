---
title: appDataDir
description: Read an application-specific data directory through the native tyrus:os module.
---

# appDataDir

`appDataDir` returns an application-specific subdirectory of [`dataDir`](./data-dir.md) for `app`.

<SignatureBlock label="Signature">appDataDir(app: string): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-app-data-dir.md -->
::: tip Never fails the program
Falls back to an empty string if it cannot be determined, rather than aborting the program.
:::
