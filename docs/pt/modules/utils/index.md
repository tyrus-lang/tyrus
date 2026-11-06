---
title: tyrus:utils
description: Helpers nativos de texto expostos pelo tyrus.
---

# tyrus:utils

`tyrus:utils` expõe helpers nativos de texto validados pelo compilador e implementados no runtime nativo.

## Imports

<SignatureBlock label="Importações">import { figlet, styleText } from "tyrus:utils";</SignatureBlock>

## Itens exportados

| Item | Assinatura | Finalidade |
|------|------------|------------|
| [`styleText`](./style-text.md) | `styleText(format, text) => string` | Aplica estilos ANSI de terminal ao texto. |
| [`figlet`](./figlet.md) | `(text: string, font?: "standard" | "small" | "big" | "slant") => string` | Renderiza texto como arte ASCII FIGlet. |

::: tip Variáveis de ambiente não são importadas de tyrus:utils
Acesso tipado ao ambiente usa `process.env` configurado por `security.envFile`; veja [Variáveis de ambiente](../../language/environment.md).
:::
