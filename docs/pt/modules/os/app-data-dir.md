---
title: appDataDir
description: Leia um diretório de dados específico de aplicação através do módulo nativo tyrus:os.
---

# appDataDir

`appDataDir` devolve um subdiretório específico de aplicação dentro de [`dataDir`](./data-dir.md) para `app`.

<SignatureBlock label="Assinatura">appDataDir(app: string): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-app-data-dir.md -->
::: tip Nunca falha o programa
Cai para uma string vazia se não puder ser determinado, em vez de abortar o programa.
:::
