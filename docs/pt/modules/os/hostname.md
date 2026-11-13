---
title: hostname
description: Leia o hostname de rede da máquina através do módulo nativo tyrus:os.
---

# hostname

`hostname` devolve o hostname de rede da máquina.

<SignatureBlock label="Assinatura">hostname(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-hostname.md -->
::: tip Nunca falha o programa
Cai para `"unknown"` se o sistema operacional não reportar um hostname, em vez de abortar o programa.
:::
