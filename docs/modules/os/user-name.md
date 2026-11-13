---
title: userName
description: Read the active user's login name through the native tyrus:os module.
---

# userName

`userName` returns the active user's login name (e.g. `"ada"`).

<SignatureBlock label="Signature">userName(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-user-name.md -->
::: tip Never fails the program
Falls back to `"unknown"` if it cannot be determined, rather than aborting the program.
:::

See also [`whoami`](./whoami.md), which returns the real name instead of the login.
