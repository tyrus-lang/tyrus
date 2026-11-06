---
title: isTTY
description: Report whether stdout is an interactive terminal through tyrus:cli.
---

# isTTY

`isTTY` reports whether stdout is attached to an interactive terminal and returns `boolean`.

<SignatureBlock label="Signature">isTTY(): boolean</SignatureBlock>

<!-- @include: ../../.fragments/modules/cli/isTTY/istty.md -->

::: tip Guard your prompts
A prompt aborts when stdin is not a TTY. Check `isTTY()` first and offer a non-interactive fallback when it returns `false`.
:::