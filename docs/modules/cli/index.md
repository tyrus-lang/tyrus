---
title: tyrus:cli
description: Native interactive terminal prompt helpers for tyrus command-line programs.
---

# tyrus:cli

`tyrus:cli` contains native prompt helpers for interactive terminal programs. Prompt UI is terminal chrome and should stay on stderr so stdout remains clean for data output.

## Imports

<SignatureBlock label="Imports">import { confirm, input, integer, isTTY, multiselect, number, password, select } from "tyrus:cli";</SignatureBlock>

## Exported Items

| Item | Signature | Purpose |
|------|-----------|---------|
| [`input`](./input.md) | `(options: InputOptions) => string` | Reads a text value. |
| [`password`](./password.md) | `(options: PasswordOptions) => string` | Reads a hidden secret value, optionally with confirmation. |
| [`confirm`](./confirm.md) | `(options: ConfirmOptions) => boolean` | Reads a yes/no answer. |
| [`select`](./select.md) | `(options: SelectOptions) => string` | Lets the user choose one option. |
| [`multiselect`](./multiselect.md) | `(options: MultiSelectOptions) => string[]` | Lets the user choose multiple options. |
| [`number`](./number.md) | `(options: NumberOptions) => f64` | Reads a floating-point number. |
| [`integer`](./integer.md) | `(options: IntegerOptions) => i64` | Reads an integer. |
| [`isTTY`](./isTTY.md) | `() => boolean` | Reports whether stdout is an interactive terminal. |

::: tip Text styling lives in tyrus:utils
Use [`styleText`](../utils/style-text.md) and [`figlet`](../utils/figlet.md) from `tyrus:utils` for banners and ANSI styling.
:::
