---
title: tyrus:os
description: Identificación multiplataforma del sistema operativo respaldada por el runtime de tyrus.
---

# tyrus:os

`tyrus:os` expone información multiplataforma sobre el sistema operativo en el que se ejecuta el binario compilado: plataforma, arquitectura, hostname, versión del kernel, locale, la constante de fin de línea, el usuario activo y directorios estándar. Es de solo lectura y puramente informativo, la información a nivel de proceso (`pid`, `cwd()`, `memoryUsage()`, etc.) vive en el global integrado `process`, documentado en [Variables de Entorno](../../language/environment.md).

## Imports

<!-- @include: ../../../.fragments/modules/os/imports.md -->

## Identificación del Sistema

| Export | Firma | Propósito |
|--------|-------|-----------|
| [`platform`](./platform.md) | `() => OsPlatform` | La plataforma objetivo. |
| [`arch`](./arch.md) | `() => OsArch` | La arquitectura de CPU objetivo. |
| [`EOL`](./eol.md) | `string` | La constante nativa de fin de línea. |
| [`hostname`](./hostname.md) | `() => string` | El hostname de red de la máquina. |
| [`release`](./release.md) | `() => string` | La versión de release/build del kernel. |
| [`systemName`](./system-name.md) | `() => string` | El nombre del sistema al estilo POSIX (`uname -s`). |
| [`locale`](./locale.md) | `() => string` | El código de locale actual. |
| [`language`](./language.md) | `() => string` | El subtag de idioma del locale actual. |

## Usuario y Directorios

| Export | Firma | Propósito |
|--------|-------|-----------|
| [`userInfo`](./user-info.md) | `() => UserInfo` | Login, directorio home y shell en una sola llamada. |
| [`userName`](./user-name.md) | `() => string` | El nombre de login. |
| [`whoami`](./whoami.md) | `() => string?` | El nombre real (GECOS), no el login. |
| [`homeDir`](./home-dir.md) | `() => string` | El directorio home del usuario. |
| [`cacheDir`](./cache-dir.md) | `() => string` | El directorio de caché del usuario. |
| [`configDir`](./config-dir.md) | `() => string` | El directorio de configuración del usuario. |
| [`dataDir`](./data-dir.md) | `() => string` | El directorio de datos del usuario. |
| [`tempDir`](./temp-dir.md) | `() => string` | El directorio temporal del sistema. |
| [`desktopDir`](./desktop-dir.md) | `() => string?` | El directorio de escritorio, si existe. |
| [`documentsDir`](./documents-dir.md) | `() => string?` | El directorio de documentos, si existe. |
| [`appDataDir`](./app-data-dir.md) | `(app: string) => string` | Un subdirectorio específico de aplicación dentro de `dataDir`. |

## Hardware y Rendimiento

| Export | Firma | Propósito |
|--------|-------|-----------|
| [`cpus`](./cpus.md) | `() => CpuInfo[]` | Una entrada por núcleo lógico de CPU. |
| [`totalMem`](./total-mem.md) | `() => u64` | Memoria total del sistema, en bytes. |
| [`freeMem`](./free-mem.md) | `() => u64` | Memoria libre del sistema, en bytes. |
| [`uptime`](./uptime.md) | `() => u64` | Segundos desde la última vez que arrancó el sistema. |
| [`loadAvg`](./load-avg.md) | `() => f64[]` | Los promedios de carga de 1/5/15 minutos. |
| [`battery`](./battery.md) | `() => BatteryInfo?` | Información de la primera batería, si existe. |
| [`cpuTemp`](./cpu-temp.md) | `(unit: string) => f32?` | La mayor temperatura de sensor reportada. |

## Pantalla y Sesión

| Export | Firma | Propósito |
|--------|-----------|---------|
| [`isDarkMode`](./is-dark-mode.md) | `() => boolean` | Si el escritorio está en modo oscuro. |
| [`terminalSize`](./terminal-size.md) | `() => [u16, u16]?` | Las columnas y filas del terminal. |
| [`isScreenLocked`](./is-screen-locked.md) | `() => boolean` | Si el salvapantallas/pantalla de bloqueo está activo. |
| [`isSessionLocked`](./is-session-locked.md) | `() => boolean` | Si la sesión del usuario actual está bloqueada. |
| [`preventSleep`](./prevent-sleep.md) | `() => Result<void, OsError>` | Inhibe la suspensión automática del sistema/pantalla. |
| [`allowSleep`](./allow-sleep.md) | `() => Result<void, OsError>` | Revierte `preventSleep`. |

`isScreenLocked`/`isSessionLocked` son de mejor esfuerzo y, por ahora, solo Linux, `false` en otras plataformas, o cuando el servicio subyacente no se puede alcanzar.

## Fuentes y Accesibilidad

| Export | Firma | Propósito |
|--------|-----------|---------|
| [`systemFonts`](./system-fonts.md) | `() => FontInfo[]` | Una entrada por cada face de fuente instalada en el sistema. |
| [`accessibleColorContrast`](./accessible-color-contrast.md) | `(fg: Color, bg: Color) => boolean` | Si un par de colores cumple la proporción de contraste AA de WCAG. |

## Utilidades

| Export | Firma | Propósito |
|--------|-----------|---------|
| [`hasCapability`](./has-capability.md) | `(cap: SystemCapability) => boolean` | Si una capacidad dada está disponible en este sistema. |

Consulta [Interfaces y Enums](./types.md) para la forma de `OsPlatform`, `OsArch`, `SystemCapability`, `UserInfo`, `CpuInfo`, `BatteryInfo`, `FontInfo`, y `Color`.

::: tip Los valores no resueltos nunca abortan el programa
`hostname()`, `release()`, `systemName()`, `userName()`, y cada función de directorio recurren a `"unknown"` o una cadena vacía si el sistema operativo no reporta un valor. `locale()` y `language()` recurren a `"und"`, el subtag estándar de "locale indeterminado". `whoami()`, `desktopDir()`, `documentsDir()`, `battery()`, y `cpuTemp()` devuelven un tipo opcional porque la ausencia es un resultado normal (un sistema headless, una cuenta sin nombre real configurado, un escritorio sin batería, un contenedor sin sensor térmico), ninguna de estas funciones puede fallar el programa como lo haría un error de filesystem. La única excepción es el argumento `unit` de `cpuTemp`: `"celsius"`/`"fahrenheit"` son los únicos valores válidos, y cualquier otro aborta el programa.
:::

## Ejemplo

<!-- @include: ../../../.fragments/modules/os/example.md -->
