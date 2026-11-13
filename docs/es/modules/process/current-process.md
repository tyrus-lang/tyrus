---
title: Proceso actual
description: Lee los argumentos y la información del proceso o establece el código de salida del programa.
---

# Proceso actual

El namespace ambiente `process` describe el programa Tyrus en ejecución. Estos valores están disponibles sin importar `tyrus:process`; ese módulo se usa para iniciar y administrar procesos hijos.

## Referencia de la API

| Miembro | Tipo o firma | Descripción |
| --- | --- | --- |
| `process.argv` | `string[]` | Argumentos de línea de comandos. El índice `0` es la ruta del ejecutable; los argumentos del usuario comienzan en el índice `1`. |
| `process.pid` | `u32` | Identificador del proceso del programa en ejecución. |
| `process.ppid` | `u32` | Identificador del proceso padre. |
| `process.cwd()` | `() => string` | Devuelve el directorio de trabajo actual. |
| `process.uptime()` | `() => u64` | Devuelve el tiempo de ejecución transcurrido en milisegundos. |
| `process.memoryUsage()` | `() => MemoryInfo` | Devuelve el uso de memoria residente en bytes. |
| `process.cpuUsage()` | `() => CpuUsage` | Devuelve el tiempo de CPU acumulado del usuario y del sistema en microsegundos. |
| `process.exit(code)` | `(code: i32) => never` | Finaliza el programa con el código indicado. |

## Argumentos de Línea de Comandos

Los argumentos después de `--` en `tyrus run` se pasan al programa, no al compilador. `Array.at()` devuelve `string?`, así que trata explícitamente los argumentos que podrían faltar.

<!-- @include: ../../../.fragments/docs/environment/argv.md -->

Como un programa Tyrus compilado es el propio ejecutable, el primer argumento del usuario está en el índice `1`. El código portado desde un runtime donde el índice `0` es un intérprete y el índice `1` es el script debe usar `slice(1)`, no `slice(2)`.

## Información del Proceso

`process.memoryUsage()` devuelve este valor:

```tyrus
interface MemoryInfo {
    readonly rss: u64;
}
```

`rss` es el tamaño residente del proceso en bytes.

`process.cpuUsage()` devuelve este valor:

```tyrus
interface CpuUsage {
    readonly user: u64;
    readonly system: u64;
}
```

`user` y `system` son el tiempo de CPU acumulado en microsegundos desde que inició el proceso, no porcentajes. Los objetivos Unix usan `getrusage`; Windows convierte los contadores de 100 nanosegundos de `GetProcessTimes` a microsegundos. Los demás objetivos devuelven `0` en ambos campos.

<!-- @include: ../../../.fragments/docs/environment/process-info.md -->

`process.exit(code)` finaliza el programa inmediatamente, así que las sentencias después de él son inalcanzables. Un programa que retorna normalmente sale con el código `0`.
