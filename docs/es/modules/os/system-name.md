---
title: systemName
description: Lee el nombre del sistema operativo al estilo POSIX mediante el módulo nativo tyrus:os.
---

# systemName

`systemName` devuelve el nombre del sistema al estilo POSIX (`uname -s`): `"Linux"`, `"Darwin"`, o `"Windows"`.

<SignatureBlock label="Firma">systemName(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-system-name.md -->
::: tip No es el nombre de la distribución Linux
`systemName` deliberadamente es el nombre POSIX del propio kernel, no el nombre de la distribución Linux (nunca `"Ubuntu"` o `"Linux Mint"`, por ejemplo). Recurre a `"unknown"` si el sistema operativo no reporta un valor, en lugar de abortar el programa.
:::
