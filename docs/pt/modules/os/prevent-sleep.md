---
title: preventSleep
description: Inibe a suspensão automática do sistema/tela através do módulo nativo tyrus:os.
---

# preventSleep

`preventSleep` inibe a suspensão automática do sistema/tela até que [`allowSleep`](./allow-sleep.md) seja chamado.

<SignatureBlock label="Assinatura">preventSleep(): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-prevent-sleep.md -->
Idempotente: chamar isso novamente enquanto a suspensão já está inibida é um sucesso sem efeito.

Implementado por plataforma:

- **Linux**, inicia `systemd-inhibit --mode=block sleep infinity` e mantém o processo filho vivo.
- **macOS**, inicia `caffeinate -dims`.
- **Windows**, chama `SetThreadExecutionState` com `ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED`; nenhum processo filho envolvido.

Retorna `Err(OsError)` (`code: "ERR_OS_SLEEP_INHIBIT_FAILED"`) se o mecanismo da plataforma não estiver disponível, por exemplo, `systemd-inhibit` ausente numa distribuição Linux sem systemd.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador estável e legível por máquina (ex.: `"ERR_OS_SLEEP_INHIBIT_FAILED"`). |
| `message` | `string` | Uma descrição legível do que deu errado. |
