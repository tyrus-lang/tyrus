---
title: systemName
description: Leia o nome do sistema operacional no padrão POSIX através do módulo nativo tyrus:os.
---

# systemName

`systemName` devolve o nome do sistema no padrão POSIX (`uname -s`): `"Linux"`, `"Darwin"`, ou `"Windows"`.

<SignatureBlock label="Assinatura">systemName(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-system-name.md -->
::: tip Não é o nome da distribuição Linux
`systemName` deliberadamente é o nome POSIX do próprio kernel, não o nome da distribuição Linux (nunca `"Ubuntu"` ou `"Linux Mint"`, por exemplo). Cai para `"unknown"` se o sistema operacional não reportar um valor, em vez de abortar o programa.
:::
