---
title: ChildProcess
description: Controla un proceso hijo y transporta sus streams estándar.
---

# ChildProcess

`ChildProcess` posee un programa externo iniciado y expone sus descriptores estándar como streams separados por capacidad.

```tyrus
interface ChildProcess {
    readonly pid: u32;
    readonly stdin: WritableStream<u8[]>;
    readonly stdout: ReadableStream<u8[]>;
    readonly stderr: ReadableStream<u8[]>;
    wait(): Promise<Result<i32, ProcessError>>;
    kill(): Promise<Result<void, ProcessError>>;
}
```

## Miembros

| Miembro | Firma | Finalidad |
| --- | --- | --- |
| `pid` | `u32` | Identificador del proceso en el sistema operativo. |
| `stdin` | `WritableStream<u8[]>` | Envía bytes a la entrada estándar. |
| `stdout` | `ReadableStream<u8[]>` | Recibe bytes de la salida estándar. |
| `stderr` | `ReadableStream<u8[]>` | Recibe bytes del error estándar. |
| [`wait()`](./wait.md) | `() => Promise<Result<i32, ProcessError>>` | Se resuelve con el código de salida o un error recuperable del proceso. |
| [`kill()`](./kill.md) | `() => Promise<Result<void, ProcessError>>` | Solicita la terminación e informa los fallos recuperables. |

## Uso y errores

<!-- @include: ../../../.fragments/modules/process/child-process.md -->

Cerrar `stdin` entrega EOF sin cerrar la salida. Los streams de salida y error permanecen legibles después de `kill()` para drenar bytes aceptados. Liberar el último handle termina y recolecta un hijo todavía en ejecución.
