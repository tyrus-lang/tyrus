---
title: hasCapability
description: Verifica si una capacidad del sistema está disponible mediante el módulo nativo tyrus:os.
---

# hasCapability

`hasCapability` verifica si una `SystemCapability` está disponible en este sistema.

<SignatureBlock label="Firma">hasCapability(cap: SystemCapability): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-has-capability.md -->
Responde "¿se puede saber?"/"¿esto funcionaría?" sin activar el efecto real, verificar `Notification` no muestra una notificación, verificar `SleepPrevention` no inicia un proceso inhibidor.

### Miembros de SystemCapability

| Miembro | Significado |
|---------|-------------|
| `Notification` | Si una notificación de escritorio podría mostrarse plausiblemente. |
| `Clipboard` | Si el portapapeles del sistema se puede alcanzar. |
| `Battery` | Si esta máquina reporta al menos una batería. |
| `DarkMode` | Si la configuración de modo oscuro del escritorio es consultable. |
| `ScreenLock` | Si el estado del salvapantallas/pantalla de bloqueo es consultable (solo Linux). |
| `SessionLock` | Si el estado de bloqueo de sesión es consultable (solo Linux). |
| `SleepPrevention` | Si `preventSleep()`/`allowSleep()` realmente funcionarían. |
| `Fonts` | Si `systemFonts()` devolvería algo. |

::: tip No es la misma pregunta que la función que refleja
`hasCapability(SystemCapability.ScreenLock)` pregunta si el estado del salvapantallas es consultable, no si la pantalla está bloqueada ahora, eso es el propio valor de retorno de [`isScreenLocked()`](./is-screen-locked.md). La misma distinción aplica a cualquier otra capacidad que refleje una función real (`Battery` vs. [`battery()`](./battery.md), `DarkMode` vs. [`isDarkMode()`](./is-dark-mode.md), y así sucesivamente).
:::
