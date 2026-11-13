---
title: release
description: Leia a versão de release/build do kernel através do módulo nativo tyrus:os.
---

# release

`release` devolve a versão de release/build do kernel (ex.: `"6.8.0-generic"`).

<SignatureBlock label="Assinatura">release(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-release.md -->
::: tip Nunca falha o programa
Cai para `"unknown"` se o sistema operacional não reportar uma versão de release, em vez de abortar o programa.
:::
