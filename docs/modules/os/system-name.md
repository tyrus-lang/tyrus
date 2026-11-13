---
title: systemName
description: Read the POSIX-style operating system name through the native tyrus:os module.
---

# systemName

`systemName` returns the POSIX-style (`uname -s`) system name: `"Linux"`, `"Darwin"`, or `"Windows"`.

<SignatureBlock label="Signature">systemName(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-system-name.md -->
::: tip Not the Linux distribution name
`systemName` is deliberately the kernel's own POSIX name, not the Linux distribution name (e.g. never `"Ubuntu"` or `"Linux Mint"`). Falls back to `"unknown"` if the operating system does not report a value, rather than aborting the program.
:::
