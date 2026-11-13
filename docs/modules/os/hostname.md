---
title: hostname
description: Read the machine's network hostname through the native tyrus:os module.
---

# hostname

`hostname` returns the machine's network hostname.

<SignatureBlock label="Signature">hostname(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-hostname.md -->
::: tip Never fails the program
Falls back to `"unknown"` if the operating system does not report a hostname, rather than aborting the program.
:::
