---
title: userName
description: Leia o nome de login do usuário ativo através do módulo nativo tyrus:os.
---

# userName

`userName` devolve o nome de login do usuário ativo (ex.: `"ada"`).

<SignatureBlock label="Assinatura">userName(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-user-name.md -->
::: tip Nunca falha o programa
Cai para `"unknown"` se não puder ser determinado, em vez de abortar o programa.
:::

Veja também [`whoami`](./whoami.md), que devolve o nome real em vez do login.
