---
title: isTTY
description: Informa si stdout es una terminal interactiva mediante tyrus:cli.
---

# isTTY

`isTTY` informa si stdout está conectado a una terminal interactiva y devuelve `boolean`.

<SignatureBlock label="Firma">isTTY(): boolean</SignatureBlock>

<!-- @include: ../../../.fragments/modules/cli/isTTY/istty.md -->

::: tip Protege tus prompts
Un prompt aborta cuando stdin no es un TTY. Consulta `isTTY()` antes y ofrece un fallback no interactivo cuando devuelva `false`.
:::