---
title: language
description: Read the current locale's language subtag through the native tyrus:os module.
---

# language

`language` returns the current locale's language subtag (e.g. `"en"`, `"pt"`), the leading part of [`locale`](./locale.md)'s full tag.

<SignatureBlock label="Signature">language(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-language.md -->
::: tip Never fails the program
Falls back to `"und"`, the standard "undetermined locale" subtag, if the OS-reported locale cannot be read, rather than aborting the program.
:::
