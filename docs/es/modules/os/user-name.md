---
title: userName
description: Lee el nombre de login del usuario activo mediante el módulo nativo tyrus:os.
---

# userName

`userName` devuelve el nombre de login del usuario activo (ej.: `"ada"`).

<SignatureBlock label="Firma">userName(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-user-name.md -->
::: tip Nunca falla el programa
Recurre a `"unknown"` si no puede determinarse, en lugar de abortar el programa.
:::

Consulta también [`whoami`](./whoami.md), que devuelve el nombre real en lugar del login.
