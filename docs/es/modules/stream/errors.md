---
title: StreamError
description: Inspecciona fallos recuperables de operaciones de stream.
---

# StreamError

`StreamError` es el tipo estructurado de error usado por operaciones recuperables de stream.

<SignatureBlock label="Firma">interface StreamError { readonly code: string; readonly message: string }</SignatureBlock>

`StreamError` es un `Error`: un `catch (error)` sin anotación lo recibe, y `error is StreamError` estrecha el binding a él, con todos los campos de `StreamError`.

## Campos

| Campo | Tipo | Finalidad |
| --- | --- | --- |
| `code` | `string` | Identificador estable para manejo programático. |
| `message` | `string` | Explicación legible. |

## Uso y errores

<!-- @include: ../../../.fragments/modules/stream/errors.md -->

Usa `throws StreamError` en una función externa cuando reenvíe una operación que puede rechazar con este error.
