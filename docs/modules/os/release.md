---
title: release
description: Read the kernel release/build version through the native tyrus:os module.
---

# release

`release` returns the kernel release/build version (e.g. `"6.8.0-generic"`).

<SignatureBlock label="Signature">release(): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-release.md -->
::: tip Never fails the program
Falls back to `"unknown"` if the operating system does not report a release version, rather than aborting the program.
:::
