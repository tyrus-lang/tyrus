---
title: whoami
description: Read the active user's real name through the native tyrus:os module.
---

# whoami

`whoami` returns the real name (not the login) of the process owner, e.g. `"Ada Lovelace"` rather than `"ada"`.

<SignatureBlock label="Signature">whoami(): string?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-whoami.md -->
::: tip Absence is a valid outcome
Returns absent (not an aborted program) if the operating system has no real name set for the account, e.g. no GECOS field, or a platform where a real name is not a distinct concept from the login.
:::

See also [`userName`](./user-name.md) and [`userInfo`](./user-info.md), which return the login name.
