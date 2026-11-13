---
title: terminalSize
description: Lee las columnas y filas del terminal mediante el módulo nativo tyrus:os.
---

# terminalSize

`terminalSize` devuelve `[columnas, filas]` del terminal.

<SignatureBlock label="Firma">terminalSize(): [u16, u16]?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-terminal-size.md -->
::: tip La ausencia es un resultado válido
Devuelve ausente, no un programa abortado, cuando la salida estándar no está conectada a un terminal, salida redirigida por pipe, a un archivo, o un proceso headless.
:::
