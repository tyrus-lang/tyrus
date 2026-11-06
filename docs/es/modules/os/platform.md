---
title: platform
description: Identifica el sistema operativo objetivo mediante el módulo nativo tyrus:os.
---

# platform

`platform` devuelve el sistema operativo objetivo como un enum nativo `OsPlatform`. Refleja el objetivo para el cual se compiló este binario, no una suposición en tiempo de ejecución.

<SignatureBlock label="Firma">platform(): OsPlatform</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-platform.md -->
### Miembros de OsPlatform

| Miembro | Significado |
|---------|-------------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Cualquier otro objetivo |
