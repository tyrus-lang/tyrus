---
title: StreamError
description: Inspecione falhas recuperáveis de operações de stream.
---

# StreamError

`StreamError` é o tipo estruturado de erro usado por operações recuperáveis de stream.

<SignatureBlock label="Assinatura">interface StreamError { readonly code: string; readonly message: string }</SignatureBlock>

`StreamError` é um `Error`: um `catch (error)` sem anotação o recebe, e `error is StreamError` estreita o binding para ele, com todos os campos de `StreamError`.

## Campos

| Campo | Tipo | Finalidade |
| --- | --- | --- |
| `code` | `string` | Identificador estável para tratamento programático. |
| `message` | `string` | Explicação legível. |

## Uso e erros

<!-- @include: ../../../.fragments/modules/stream/errors.md -->

Use `throws StreamError` em uma função externa quando ela encaminhar uma operação que pode rejeitar com esse erro.
