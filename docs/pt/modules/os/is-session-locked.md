---
title: isSessionLocked
description: Detecta se a sessão do usuário atual está bloqueada através do módulo nativo tyrus:os.
---

# isSessionLocked

`isSessionLocked` retorna se a sessão do usuário atual está bloqueada.

<SignatureBlock label="Assinatura">isSessionLocked(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-session-locked.md -->
::: tip Melhor esforço, só Linux por enquanto
Consulta a propriedade `LockedHint` por sessão do `systemd-logind` via `loginctl`, a flag de bloqueio em nível de sessão rastreada por todo desktop que registra uma sessão com o `logind` (GNOME, KDE e a maioria dos outros no Linux). Retorna `false` em qualquer outra plataforma, ou quando o estado da sessão não pode ser alcançado.
:::

Veja [`isScreenLocked`](./is-screen-locked.md) para o estado relacionado, porém distinto, de proteção de tela.
