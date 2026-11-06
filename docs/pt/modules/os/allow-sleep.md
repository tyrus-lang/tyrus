---
title: allowSleep
description: Reverte preventSleep através do módulo nativo tyrus:os.
---

# allowSleep

`allowSleep` reverte [`preventSleep`](./prevent-sleep.md), deixando o sistema suspender normalmente de novo.

<SignatureBlock label="Assinatura">allowSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-allow-sleep.md -->
Idempotente: chamar isso sem nada inibido é um sucesso sem efeito, no mesmo espírito da idempotência de `preventSleep`. Retorna `Err(OsError)` com o código `ERR_OS_SLEEP_ALLOW_FAILED` se a API da plataforma não conseguir restaurar o comportamento normal de suspensão.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador estável e legível por máquina. |
| `message` | `string` | Uma descrição legível do que deu errado. |
