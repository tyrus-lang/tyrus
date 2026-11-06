---
title: isScreenLocked
description: Detecta si el salvapantallas/pantalla de bloqueo está activo mediante el módulo nativo tyrus:os.
---

# isScreenLocked

`isScreenLocked` devuelve si el salvapantallas/pantalla de bloqueo está activo en este momento.

<SignatureBlock label="Firma">isScreenLocked(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-screen-locked.md -->
::: tip Mejor esfuerzo, solo Linux por ahora
Consulta la interfaz D-Bus de salvapantallas de freedesktop (`org.freedesktop.ScreenSaver.GetActive`, implementada por GNOME, KDE y la mayoría de los demás salvapantallas/bloqueadores de Linux). Devuelve `false` en cualquier otra plataforma, o cuando el servicio de salvapantallas por D-Bus no se puede alcanzar, una máquina headless, sin bus de sesión.
:::

Consulta [`isSessionLocked`](./is-session-locked.md) para el estado relacionado, pero distinto, de bloqueo de sesión.
