---
title: cacheDir
description: Leia o diretório de cache do usuário através do módulo nativo tyrus:os.
---

# cacheDir

`cacheDir` devolve o diretório de cache padrão da plataforma para o usuário (ex.: `~/.cache` no Linux).

<SignatureBlock label="Assinatura">cacheDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cache-dir.md -->
::: tip Nunca falha o programa
Cai para uma string vazia se não puder ser determinado, em vez de abortar o programa.
:::
