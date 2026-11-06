---
title: platform
description: Identifique o sistema operacional alvo através do módulo nativo tyrus:os.
---

# platform

`platform` devolve o sistema operacional alvo como um enum nativo `OsPlatform`. Ele reflete o alvo para o qual este binário foi compilado, não um palpite em tempo de execução.

<SignatureBlock label="Assinatura">platform(): OsPlatform</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-platform.md -->
### Membros de OsPlatform

| Membro | Significado |
|--------|-------------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Qualquer outro alvo |
