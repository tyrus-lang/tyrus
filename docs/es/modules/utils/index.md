---
title: tyrus:utils
description: Helpers nativos de texto expuestos por tyrus.
---

# tyrus:utils

`tyrus:utils` expone helpers nativos de texto validados por el compilador e implementados en el runtime nativo.

## Imports

<SignatureBlock label="Importaciones">import { figlet, styleText } from "tyrus:utils";</SignatureBlock>

## Ítems exportados

| Ítem | Firma | Finalidad |
|------|-------|-----------|
| [`styleText`](./style-text.md) | `styleText(format, text) => string` | Aplica estilos ANSI de terminal al texto. |
| [`figlet`](./figlet.md) | `(text: string, font?: "standard" | "small" | "big" | "slant") => string` | Renderiza texto como arte ASCII FIGlet. |

::: tip Las variables de entorno no se importan desde tyrus:utils
El acceso tipado al entorno usa `process.env` configurado mediante `security.envFile`; consulta [Variables de entorno](../../language/environment.md).
:::
