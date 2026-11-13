---
title: terminalSize
description: Read the terminal's columns and rows through the native tyrus:os module.
---

# terminalSize

`terminalSize` returns the terminal's `[columns, rows]`.

<SignatureBlock label="Signature">terminalSize(): [u16, u16]?</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-terminal-size.md -->
::: tip Absence is a valid outcome
Returns absent, not an aborted program, when stdout is not attached to a terminal, piped output, redirected to a file, or a headless process.
:::
