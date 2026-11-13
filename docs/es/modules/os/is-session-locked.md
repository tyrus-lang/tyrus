---
title: isSessionLocked
description: Detecta si la sesión del usuario actual está bloqueada mediante el módulo nativo tyrus:os.
---

# isSessionLocked

`isSessionLocked` devuelve si la sesión del usuario actual está bloqueada.

<SignatureBlock label="Firma">isSessionLocked(): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-is-session-locked.md -->
::: tip Mejor esfuerzo, solo Linux por ahora
Consulta la propiedad `LockedHint` por sesión de `systemd-logind` mediante `loginctl`, la marca de bloqueo a nivel de sesión que rastrea todo escritorio que registra una sesión con `logind` (GNOME, KDE y la mayoría de los demás en Linux). Devuelve `false` en cualquier otra plataforma, o cuando el estado de la sesión no se puede alcanzar.
:::

Consulta [`isScreenLocked`](./is-screen-locked.md) para el estado relacionado, pero distinto, del salvapantallas.
