---
title: release
description: Lee la versión de release/build del kernel mediante el módulo nativo tyrus:os.
---

# release

`release` devuelve la versión de release/build del kernel (ej.: `"6.8.0-generic"`).

<SignatureBlock label="Firma">release(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-release.md -->
::: tip Nunca falla el programa
Recurre a `"unknown"` si el sistema operativo no reporta una versión de release, en lugar de abortar el programa.
:::
