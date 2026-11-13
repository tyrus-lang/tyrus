---
title: dataDir
description: Leia o diretório de dados do usuário através do módulo nativo tyrus:os.
---

# dataDir

`dataDir` devolve o diretório de dados padrão da plataforma para o usuário (ex.: `~/.local/share` no Linux).

<SignatureBlock label="Assinatura">dataDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-data-dir.md -->
::: tip Nunca falha o programa
Cai para uma string vazia se não puder ser determinado, em vez de abortar o programa.
:::

Veja também [`appDataDir`](./app-data-dir.md) para um subdiretório específico da aplicação.
