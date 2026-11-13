---
title: tyrus:cli
description: Helpers nativos de prompt interactivo para programas CLI en tyrus.
---

# tyrus:cli

`tyrus:cli` contiene helpers nativos de prompt para programas interactivos de terminal. La UI de prompt es chrome de terminal y debe quedarse en stderr para mantener stdout limpio para salida de datos.

## Imports

<SignatureBlock label="Importaciones">import { confirm, input, integer, isTTY, multiselect, number, password, select } from "tyrus:cli";</SignatureBlock>

## Ítems exportados

| Ítem | Firma | Finalidad |
|------|-------|-----------|
| [`input`](./input.md) | `(options: InputOptions) => string` | Lee un valor de texto. |
| [`password`](./password.md) | `(options: PasswordOptions) => string` | Lee un valor secreto oculto, opcionalmente con confirmación. |
| [`confirm`](./confirm.md) | `(options: ConfirmOptions) => boolean` | Lee una respuesta sí/no. |
| [`select`](./select.md) | `(options: SelectOptions) => string` | Permite elegir una opción. |
| [`multiselect`](./multiselect.md) | `(options: MultiSelectOptions) => string[]` | Permite elegir múltiples opciones. |
| [`number`](./number.md) | `(options: NumberOptions) => f64` | Lee un número de punto flotante. |
| [`integer`](./integer.md) | `(options: IntegerOptions) => i64` | Lee un entero. |
| [`isTTY`](./isTTY.md) | `() => boolean` | Informa si stdout es una terminal interactiva. |

::: tip El estilo de texto queda en tyrus:utils
Usa [`styleText`](../utils/style-text.md) y [`figlet`](../utils/figlet.md) de `tyrus:utils` para banners y estilo ANSI.
:::
