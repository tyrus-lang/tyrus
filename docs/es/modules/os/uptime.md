---
title: uptime
description: Lee el uptime del sistema mediante el módulo nativo tyrus:os.
---

# uptime

`uptime` devuelve cuántos segundos han pasado desde que el sistema en sí arrancó por última vez.

<SignatureBlock label="Firma">uptime(): u64</SignatureBlock>

<!-- @include: ../../../.fragments/modules/os/usage-uptime.md -->

::: tip No es el uptime del proceso actual
Esto es el uptime del sistema, no del programa en ejecución. Para cuánto tiempo lleva ejecutándose el proceso actual, usa [`process.uptime()`](../../language/environment.md).
:::
