---
title: hostname
description: Lee el hostname de red de la máquina mediante el módulo nativo tyrus:os.
---

# hostname

`hostname` devuelve el hostname de red de la máquina.

<SignatureBlock label="Firma">hostname(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-hostname.md -->
::: tip Nunca falla el programa
Recurre a `"unknown"` si el sistema operativo no reporta un hostname, en lugar de abortar el programa.
:::
