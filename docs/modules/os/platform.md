---
title: platform
description: Identify the target operating system through the native tyrus:os module.
---

# platform

`platform` returns the target operating system as a native `OsPlatform` enum. It reflects the target this binary was compiled for, not a runtime guess.

<SignatureBlock label="Signature">platform(): OsPlatform</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-platform.md -->
### OsPlatform Members

| Member | Meaning |
|--------|---------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Any other target |
