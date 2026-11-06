---
title: homeDir
description: Leia o diretório home do usuário através do módulo nativo tyrus:os.
---

# homeDir

`homeDir` devolve o diretório home do usuário.

<SignatureBlock label="Assinatura">homeDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-home-dir.md -->
::: tip Nunca falha o programa
Cai para uma string vazia se não puder ser determinado, em vez de abortar o programa.
:::
