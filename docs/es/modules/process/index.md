---
title: Procesos
description: Inicia programas externos y administra su entrada, salida y estado de finalización.
---

# Procesos

El módulo `tyrus:process` inicia programas externos sin shell. Cada proceso hijo expone streams limitados e independientes para la entrada, la salida y el error estándar.

## Importación

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";
```

`exec(command, args?)` pasa el ejecutable y cada argumento directamente al sistema operativo. No interpreta comillas de shell, pipes, redirecciones ni otra sintaxis de shell.

## Referencia de la API

| Nombre | Tipo | Descripción |
| --- | --- | --- |
| [`exec`](./exec.md) | Función | Inicia un proceso hijo y devuelve `Result<ChildProcess, ProcessError>`. |
| [`ChildProcess`](./child-process.md) | Interfaz | Proporciona el ID del proceso, los streams estándar y los métodos `wait()` y `kill()`. |
| [`wait()`](./wait.md) | Método | Espera a que termine el proceso hijo y devuelve su código de salida. |
| [`kill()`](./kill.md) | Método | Solicita la terminación de un proceso hijo en ejecución. |
| [`ProcessError`](./errors.md) | Interfaz | Describe fallos recuperables de `exec`, `wait()` y `kill()`. |
| [Proceso actual](./current-process.md) | API ambiente | Documenta `process.argv`, la información del proceso y `process.exit()`. |

## Streams del Proceso Hijo

`stdin` es un `WritableStream<u8[]>`; ciérralo con `await child.stdin.close()` para enviar EOF. `stdout` y `stderr` son valores independientes de `ReadableStream<u8[]>`. Los métodos de estos streams están documentados en el módulo [`tyrus:stream`](../stream/).

Cada stream del proceso usa un buffer limitado de 256 KiB y produce chunks de como máximo 64 KiB. Un consumidor lento pausa la lectura de ese descriptor en vez de acumular la salida en memoria. `stdout` y `stderr` preservan todos los bytes, incluidos los bytes cero.

Liberar el último handle de `ChildProcess` termina y recolecta un hijo que sigue en ejecución. Un código de salida distinto de cero y los bytes escritos en `stderr` son resultados normales del proceso, no valores `ProcessError`.

::: warning Disponibilidad en el playground
`tyrus:process` requiere acceso a procesos del sistema operativo y no está disponible en compilaciones sandboxed.
:::
