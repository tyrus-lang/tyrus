---
title: desktopDir
description: Lee el directorio de escritorio del usuario mediante el módulo nativo tyrus:os.
---

# desktopDir

`desktopDir` devuelve el directorio de escritorio del usuario.

<SignatureBlock label="Firma">desktopDir(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-desktop-dir.md -->
::: tip La ausencia es un resultado válido
Un sistema headless (servidor, contenedor, CI) comúnmente no tiene ningún directorio de escritorio, eso es un resultado normal, no un fallo.
:::
