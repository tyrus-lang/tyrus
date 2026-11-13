---
title: isTTY
description: Informa se stdout é um terminal interativo por meio de tyrus:cli.
---

# isTTY

`isTTY` informa se stdout está conectado a um terminal interativo e retorna `boolean`.

<SignatureBlock label="Assinatura">isTTY(): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/cli/isTTY/istty.md -->

::: tip Proteja seus prompts
Um prompt aborta quando stdin não é um TTY. Consulte `isTTY()` antes e ofereça um fallback não interativo quando ela retornar `false`.
:::