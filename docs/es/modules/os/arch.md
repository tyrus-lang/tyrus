---
title: arch
description: Identifica la arquitectura de CPU objetivo mediante el módulo nativo tyrus:os.
---

# arch

`arch` devuelve la arquitectura de CPU objetivo como un enum nativo `OsArch`. Refleja el objetivo para el cual se compiló este binario, no una suposición en tiempo de ejecución.

<SignatureBlock label="Firma">arch(): OsArch</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-arch.md -->
### Miembros de OsArch

| Miembro | Significado |
|---------|-------------|
| `X86_64` | x86 de 64 bits |
| `Aarch64` | ARM de 64 bits |
| `X86` | x86 de 32 bits |
| `Arm` | ARM de 32 bits |
| `Other` | Cualquier otro objetivo |
