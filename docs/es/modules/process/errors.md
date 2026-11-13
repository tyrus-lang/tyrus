---
title: ProcessError
description: Inspecciona fallos recuperables de operaciones de proceso.
---

# ProcessError

`ProcessError` es el tipo estructurado de error para fallos recuperables de proceso.

<SignatureBlock label="Firma">interface ProcessError { readonly code: string; readonly message: string }</SignatureBlock>

`ProcessError` es un valor `Error` y puede ser lanzado por funciones definidas por el usuario. La API nativa `exec` lo devuelve en la variante `Err` de `Result<ChildProcess, ProcessError>`.

## Campos

| Campo | Tipo | Finalidad |
| --- | --- | --- |
| `code` | `string` | Identificador estable `ERR_PROCESS_*`. |
| `message` | `string` | Explicación legible. |

## Uso y errores

<!-- @include: ../../../.fragments/modules/process/errors.md -->

Los fallos de inicio incluyen argumentos inválidos, pipes no disponibles, un archivo que el programa no tiene permiso para ejecutar (`ERR_PROCESS_PERMISSION_DENIED`) y un ejecutable que no puede iniciar por cualquier otro motivo, como un comando que no existe (`ERR_PROCESS_SPAWN_FAILED`). Un código de salida distinto de cero y bytes en `stderr` son resultados normales, no valores `ProcessError`.
