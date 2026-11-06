---
title: tyrus:utils
description: Native text utility helpers exposed by tyrus.
---

# tyrus:utils

`tyrus:utils` exposes native text helpers that are validated by the compiler and implemented in the native runtime.

## Imports

<SignatureBlock label="Imports">import { figlet, styleText } from "tyrus:utils";</SignatureBlock>

## Exported Items

| Item | Signature | Purpose |
|------|-----------|---------|
| [`styleText`](./style-text.md) | `styleText(format, text) => string` | Applies ANSI terminal styles to text. |
| [`figlet`](./figlet.md) | `(text: string, font?: "standard" | "small" | "big" | "slant") => string` | Renders text as FIGlet ASCII art. |

::: tip Environment variables are not imported from tyrus:utils
Typed environment access uses `process.env` configured through `security.envFile`; see [Environment Variables](../../language/environment.md).
:::
