---
title: arch
description: Identify the target CPU architecture through the native tyrus:os module.
---

# arch

`arch` returns the target CPU architecture as a native `OsArch` enum. It reflects the target this binary was compiled for, not a runtime guess.

<SignatureBlock label="Signature">arch(): OsArch</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-arch.md -->
### OsArch Members

| Member | Meaning |
|--------|---------|
| `X86_64` | 64-bit x86 |
| `Aarch64` | 64-bit ARM |
| `X86` | 32-bit x86 |
| `Arm` | 32-bit ARM |
| `Other` | Any other target |
