---
title: notify
description: Muestra una notificación nativa del sistema mediante el módulo nativo tyrus:os.
---

# notify

`notify` solicita una notificación nativa del sistema y devuelve un error estructurado si el servicio de la plataforma no puede mostrarla.

<SignatureBlock label="Firma">notify(title: string, body: string, options?: NotifyOptions): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-notify.md -->
`options` asume los valores predeterminados de la propia plataforma para cada campo cuando se omite.

### Campos de NotifyOptions

| Campo | Tipo | Significado |
|-------|------|-------------|
| `icon` | `string?` | El nombre o ruta de un ícono para mostrar junto a la notificación. |
| `urgency` | `NotifyUrgency?` | Consulta [NotifyUrgency](#miembros-de-notifyurgency). El valor predeterminado es `Normal`. |
| `timeout` | `u32?` | Cuánto tiempo permanece visible la notificación, en milisegundos, antes de que el entorno de escritorio la descarte automáticamente. |

### Miembros de NotifyUrgency

| Miembro | Significado |
|---------|-------------|
| `Low` | Prioridad baja, el entorno de escritorio puede no mostrarla en modo "no molestar". |
| `Normal` | Prioridad predeterminada. |
| `Critical` | Prioridad alta, algunos entornos de escritorio la mantienen en pantalla hasta que se descarta. |

Devuelve `Err(OsError)` con el código `ERR_OS_NOTIFICATION_FAILED` cuando el servicio de notificaciones de la plataforma no está disponible o rechaza la notificación. Un título o cuerpo nulo devuelve `ERR_OS_INVALID_ARGUMENT`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador de error estable y legible por máquina. |
| `message` | `string` | Una descripción legible del error. |
:::
