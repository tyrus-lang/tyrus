---
title: arch
description: Identifique a arquitetura de CPU alvo através do módulo nativo tyrus:os.
---

# arch

`arch` devolve a arquitetura de CPU alvo como um enum nativo `OsArch`. Ele reflete o alvo para o qual este binário foi compilado, não um palpite em tempo de execução.

<SignatureBlock label="Assinatura">arch(): OsArch</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-arch.md -->
### Membros de OsArch

| Membro | Significado |
|--------|-------------|
| `X86_64` | x86 64-bit |
| `Aarch64` | ARM 64-bit |
| `X86` | x86 32-bit |
| `Arm` | ARM 32-bit |
| `Other` | Qualquer outro alvo |
