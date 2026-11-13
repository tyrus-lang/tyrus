---
title: preventSleep
description: Inhibe la suspensión automática del sistema/pantalla mediante el módulo nativo tyrus:os.
---

# preventSleep

`preventSleep` inhibe la suspensión automática del sistema/pantalla hasta que se llame a [`allowSleep`](./allow-sleep.md).

<SignatureBlock label="Firma">preventSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-prevent-sleep.md -->
Idempotente: llamar a esto de nuevo mientras la suspensión ya está inhibida es un éxito sin efecto.

Implementado por plataforma:

- **Linux**, inicia `systemd-inhibit --mode=block sleep infinity` y mantiene vivo el proceso hijo.
- **macOS**, inicia `caffeinate -dims`.
- **Windows**, llama a `SetThreadExecutionState` con `ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED`; no hay proceso hijo involucrado.

Devuelve `Err(OsError)` (`code: "ERR_OS_SLEEP_INHIBIT_FAILED"`) si el mecanismo de la plataforma no está disponible, por ejemplo, `systemd-inhibit` ausente en una distribución Linux sin systemd.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador estable y legible por máquina (ej.: `"ERR_OS_SLEEP_INHIBIT_FAILED"`). |
| `message` | `string` | Una descripción legible de lo que salió mal. |
