---
title: terminalSize
description: Lê as colunas e linhas do terminal através do módulo nativo tyrus:os.
---

# terminalSize

`terminalSize` retorna `[colunas, linhas]` do terminal.

<SignatureBlock label="Assinatura">terminalSize(): [u16, u16]?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-terminal-size.md -->
::: tip Ausência é um resultado válido
Retorna ausente, não um programa abortado, quando a saída padrão não está conectada a um terminal, saída redirecionada por pipe, para um arquivo, ou um processo headless.
:::
