---
title: cacheDir
description: Lee el directorio de caché del usuario mediante el módulo nativo tyrus:os.
---

# cacheDir

`cacheDir` devuelve el directorio de caché estándar de la plataforma para el usuario (ej.: `~/.cache` en Linux).

<SignatureBlock label="Firma">cacheDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-cache-dir.md -->
::: tip Nunca falla el programa
Recurre a una cadena vacía si no puede determinarse, en lugar de abortar el programa.
:::
