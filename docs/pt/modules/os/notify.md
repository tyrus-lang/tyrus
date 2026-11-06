---
title: notify
description: Mostre uma notificação nativa do sistema através do módulo nativo tyrus:os.
---

# notify

`notify` solicita uma notificação nativa do sistema e retorna um erro estruturado se o serviço da plataforma não conseguir exibi-la.

<SignatureBlock label="Assinatura">notify(title: string, body: string, options?: NotifyOptions): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-notify.md -->
`options` assume os padrões da própria plataforma para cada campo quando omitido.

### Campos de NotifyOptions

| Campo | Tipo | Significado |
|-------|------|-------------|
| `icon` | `string?` | O nome ou caminho de um ícone para mostrar junto da notificação. |
| `urgency` | `NotifyUrgency?` | Veja [NotifyUrgency](#membros-de-notifyurgency). Padrão é `Normal`. |
| `timeout` | `u32?` | Por quanto tempo a notificação fica visível, em milissegundos, antes do ambiente de desktop dispensá-la automaticamente. |

### Membros de NotifyUrgency

| Membro | Significado |
|--------|-------------|
| `Low` | Prioridade baixa, o ambiente de desktop pode nem mostrar em modo "não perturbe". |
| `Normal` | Prioridade padrão. |
| `Critical` | Prioridade alta, alguns ambientes de desktop mantêm na tela até ser dispensada. |

Retorna `Err(OsError)` com o código `ERR_OS_NOTIFICATION_FAILED` quando o serviço de notificações da plataforma está indisponível ou recusa a notificação. Um título ou corpo nulo retorna `ERR_OS_INVALID_ARGUMENT`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador de erro estável e legível por máquina. |
| `message` | `string` | Uma descrição legível da falha. |
:::
