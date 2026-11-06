---
title: isDarkMode
description: Detecta si el escritorio está en modo oscuro mediante el módulo nativo tyrus:os.
---

# isDarkMode

`isDarkMode` devuelve si el escritorio está actualmente en modo oscuro.

<SignatureBlock label="Firma">isDarkMode(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-dark-mode.md -->
::: tip Mejor esfuerzo
Depende de que la plataforma tenga una configuración de modo oscuro consultable (el registro de Windows, el `AppleInterfaceStyle` de macOS, o un portal de escritorio/configuración de GNOME en Linux). Cuando ninguna es alcanzable, una máquina headless, un escritorio no compatible, esto devuelve `false` (claro) de forma conservadora, en lugar de abortar el programa.
:::
