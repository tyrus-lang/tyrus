---
title: locale
description: Read the current locale tag through the native tyrus:os module.
---

# locale

`locale` returns the current locale tag (e.g. `"en-US"`, `"pt-BR"`).

<SignatureBlock label="Signature">locale(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-locale.md -->
::: tip Never fails the program
Falls back to `"und"`, the standard "undetermined locale" subtag, if the OS-reported locale cannot be read, rather than aborting the program.
:::

See also [`language`](./language.md), which returns just the leading language subtag.
