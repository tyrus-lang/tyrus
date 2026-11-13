---
title: ProcessError
description: Inspecione falhas recuperáveis de operações de processo.
---

# ProcessError

`ProcessError` é o tipo estruturado de erro para falhas recuperáveis de processo.

<SignatureBlock label="Assinatura">interface ProcessError { readonly code: string; readonly message: string }</SignatureBlock>

`ProcessError` é um valor `Error` e pode ser lançado por funções definidas pelo usuário. A API nativa `exec` o retorna na variante `Err` de `Result<ChildProcess, ProcessError>`.

## Campos

| Campo | Tipo | Finalidade |
| --- | --- | --- |
| `code` | `string` | Identificador estável `ERR_PROCESS_*`. |
| `message` | `string` | Explicação legível. |

## Uso e erros

<!-- @include: ../../../.fragments/modules/process/errors.md -->

Falhas de início incluem argumentos inválidos, pipes indisponíveis, um arquivo que o programa não tem permissão de executar (`ERR_PROCESS_PERMISSION_DENIED`) e um executável que não pode iniciar por qualquer outro motivo, como um comando que não existe (`ERR_PROCESS_SPAWN_FAILED`). Código de saída diferente de zero e bytes em `stderr` são resultados normais, não valores `ProcessError`.
