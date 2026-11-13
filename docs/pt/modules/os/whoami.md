---
title: whoami
description: Leia o nome real do usuário ativo através do módulo nativo tyrus:os.
---

# whoami

`whoami` devolve o nome real (não o login) do dono do processo, ex.: `"Ada Lovelace"` em vez de `"ada"`.

<SignatureBlock label="Assinatura">whoami(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-whoami.md -->
::: tip Ausência é um resultado válido
Devolve ausência (não um programa abortado) se o sistema operacional não tiver um nome real configurado para a conta, ex.: sem campo GECOS, ou uma plataforma onde nome real não é um conceito distinto do login.
:::

Veja também [`userName`](./user-name.md) e [`userInfo`](./user-info.md), que devolvem o nome de login.
