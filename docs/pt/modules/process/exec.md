---
title: exec
description: Inicie um programa externo sem invocar um shell.
---

# exec

`exec` inicia um programa externo e conecta seus descritores padrão a streams de bytes limitados.

<SignatureBlock label="Assinatura">exec(command: string, args?: string[]): Result&lt;ChildProcess, ProcessError&gt;</SignatureBlock>

## Parâmetros

| Parâmetro | Tipo | Finalidade |
| --- | --- | --- |
| `command` | `string` | Caminho ou nome do executável. |
| `args` | `string[]?` | Argumentos individuais ou uma lista vazia omitida. |

## Uso e erros

<!-- @include: ../../../.fragments/modules/process/exec.md -->

Cada argumento é passado diretamente ao programa. `exec` não interpreta aspas, redirecionamentos ou pipelines de shell. Uma falha ao iniciar retorna `Err(ProcessError)`.
