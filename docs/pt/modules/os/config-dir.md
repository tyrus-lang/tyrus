---
title: configDir
description: Leia o diretório de configuração do usuário através do módulo nativo tyrus:os.
---

# configDir

`configDir` devolve o diretório de configuração padrão da plataforma para o usuário (ex.: `~/.config` no Linux).

<SignatureBlock label="Assinatura">configDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-config-dir.md -->
::: tip Nunca falha o programa
Cai para uma string vazia se não puder ser determinado, em vez de abortar o programa.
:::
