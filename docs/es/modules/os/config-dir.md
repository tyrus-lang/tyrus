---
title: configDir
description: Lee el directorio de configuración del usuario mediante el módulo nativo tyrus:os.
---

# configDir

`configDir` devuelve el directorio de configuración estándar de la plataforma para el usuario (ej.: `~/.config` en Linux).

<SignatureBlock label="Firma">configDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-config-dir.md -->
::: tip Nunca falla el programa
Recurre a una cadena vacía si no puede determinarse, en lugar de abortar el programa.
:::
