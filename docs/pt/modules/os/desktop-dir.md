---
title: desktopDir
description: Leia o diretório da área de trabalho do usuário através do módulo nativo tyrus:os.
---

# desktopDir

`desktopDir` devolve o diretório da área de trabalho do usuário.

<SignatureBlock label="Assinatura">desktopDir(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-desktop-dir.md -->
::: tip Ausência é um resultado válido
Um sistema headless (servidor, container, CI) comumente não tem diretório de área de trabalho nenhum, isso é um resultado normal, não uma falha.
:::
