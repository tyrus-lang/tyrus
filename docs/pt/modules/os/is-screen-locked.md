---
title: isScreenLocked
description: Detecta se a proteção de tela/tela de bloqueio está ativa através do módulo nativo tyrus:os.
---

# isScreenLocked

`isScreenLocked` retorna se a proteção de tela/tela de bloqueio está ativa no momento.

<SignatureBlock label="Assinatura">isScreenLocked(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-screen-locked.md -->
::: tip Melhor esforço, só Linux por enquanto
Consulta a interface D-Bus de proteção de tela do freedesktop (`org.freedesktop.ScreenSaver.GetActive`, implementada pelo GNOME, KDE e a maioria dos outros protetores de tela/bloqueadores do Linux). Retorna `false` em qualquer outra plataforma, ou quando o serviço de proteção de tela via D-Bus não pode ser alcançado, uma máquina headless, sem barramento de sessão.
:::

Veja [`isSessionLocked`](./is-session-locked.md) para o estado relacionado, porém distinto, de bloqueio de sessão.
