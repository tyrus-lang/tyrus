---
title: dataDir
description: Lee el directorio de datos del usuario mediante el módulo nativo tyrus:os.
---

# dataDir

`dataDir` devuelve el directorio de datos estándar de la plataforma para el usuario (ej.: `~/.local/share` en Linux).

<SignatureBlock label="Firma">dataDir(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-data-dir.md -->
::: tip Nunca falla el programa
Recurre a una cadena vacía si no puede determinarse, en lugar de abortar el programa.
:::

Consulta también [`appDataDir`](./app-data-dir.md) para un subdirectorio específico de la aplicación.
