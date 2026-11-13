---
title: uptime
description: Leia o uptime do sistema através do módulo nativo tyrus:os.
---

# uptime

`uptime` devolve há quantos segundos o sistema em si foi ligado pela última vez.

<SignatureBlock label="Assinatura">uptime(): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/os/usage-uptime.md -->

::: tip Não é o uptime do processo atual
Isto é o uptime do sistema, não do programa em execução. Para há quanto tempo o processo atual está rodando, use [`process.uptime()`](../../language/environment.md).
:::
