---
title: exec
description: Inicia un programa externo sin invocar un shell.
---

# exec

`exec` inicia un programa externo y conecta sus descriptores estándar a streams de bytes limitados.

<SignatureBlock label="Firma">exec(command: string, args?: string[]): Result&lt;ChildProcess, ProcessError&gt;</SignatureBlock>

## Parámetros

| Parámetro | Tipo | Finalidad |
| --- | --- | --- |
| `command` | `string` | Ruta o nombre del ejecutable. |
| `args` | `string[]?` | Argumentos individuales o una lista vacía omitida. |

## Uso y errores

<!-- @include: ../../../.fragments/modules/process/exec.md -->

Cada argumento se pasa directamente al programa. `exec` no interpreta comillas, redirecciones ni pipelines de shell. Un fallo al iniciar devuelve `Err(ProcessError)`.
