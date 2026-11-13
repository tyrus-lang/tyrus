---
title: allowSleep
description: Revierte preventSleep mediante el módulo nativo tyrus:os.
---

# allowSleep

`allowSleep` revierte [`preventSleep`](./prevent-sleep.md), dejando que el sistema vuelva a suspenderse normalmente.

<SignatureBlock label="Firma">allowSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-allow-sleep.md -->
Idempotente: llamar a esto sin nada inhibido es un éxito sin efecto, en el mismo espíritu que la idempotencia de `preventSleep`. Devuelve `Err(OsError)` con el código `ERR_OS_SLEEP_ALLOW_FAILED` si la API de la plataforma no puede restaurar el comportamiento normal de suspensión.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador estable y legible por máquina. |
| `message` | `string` | Una descripción legible de lo que salió mal. |
