---
title: homeDir
description: Lee el directorio home del usuario mediante el módulo nativo tyrus:os.
---

# homeDir

`homeDir` devuelve el directorio home del usuario.

<SignatureBlock label="Firma">homeDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-home-dir.md -->
::: tip Nunca falla el programa
Recurre a una cadena vacía si no puede determinarse, en lugar de abortar el programa.
:::
