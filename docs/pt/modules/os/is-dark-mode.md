---
title: isDarkMode
description: Detecta se a área de trabalho está em modo escuro através do módulo nativo tyrus:os.
---

# isDarkMode

`isDarkMode` retorna se a área de trabalho está atualmente em modo escuro.

<SignatureBlock label="Assinatura">isDarkMode(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-dark-mode.md -->
::: tip Melhor esforço
Depende de a plataforma ter uma configuração de modo escuro consultável (o registro do Windows, o `AppleInterfaceStyle` do macOS, ou um portal de desktop/configuração do GNOME no Linux). Quando nenhuma delas está acessível, uma máquina headless, um desktop não suportado, isso retorna `false` (claro) de forma conservadora, em vez de abortar o programa.
:::
