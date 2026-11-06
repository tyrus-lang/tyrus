---
title: tyrus:cli
description: Helpers nativos de prompt interativo para programas CLI em tyrus.
---

# tyrus:cli

`tyrus:cli` contém helpers nativos de prompt para programas interativos de terminal. UI de prompt é chrome de terminal e deve ficar em stderr para manter stdout limpo para saída de dados.

## Imports

<SignatureBlock label="Importações">import { confirm, input, integer, isTTY, multiselect, number, password, select } from "tyrus:cli";</SignatureBlock>

## Itens exportados

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`input`](./input.md) | `(options: InputOptions) => string` | Lê um valor de texto. |
| [`password`](./password.md) | `(options: PasswordOptions) => string` | Lê um valor secreto oculto, opcionalmente com confirmação. |
| [`confirm`](./confirm.md) | `(options: ConfirmOptions) => boolean` | Lê uma resposta sim/não. |
| [`select`](./select.md) | `(options: SelectOptions) => string` | Permite escolher uma opção. |
| [`multiselect`](./multiselect.md) | `(options: MultiSelectOptions) => string[]` | Permite escolher múltiplas opções. |
| [`number`](./number.md) | `(options: NumberOptions) => f64` | Lê um número de ponto flutuante. |
| [`integer`](./integer.md) | `(options: IntegerOptions) => i64` | Lê um inteiro. |
| [`isTTY`](./isTTY.md) | `() => boolean` | Informa se stdout é um terminal interativo. |

::: tip Estilo de texto fica em tyrus:utils
Use [`styleText`](../utils/style-text.md) e [`figlet`](../utils/figlet.md) de `tyrus:utils` para banners e estilo ANSI.
:::
