---
title: openUrl
description: Abre una URL con el manejador predeterminado de la plataforma mediante el módulo nativo tyrus:os.
---

# openUrl

`openUrl` abre `url` con el manejador predeterminado de la plataforma (navegador, cliente de correo, etc.), el mismo efecto que hacer doble clic en un enlace.

<SignatureBlock label="Firma">openUrl(url: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-open-url.md -->
Devuelve `Err(OsError)` (`code: "ERR_OS_NO_HANDLER"`) si no hay ninguna aplicación registrada para abrir `url`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador estable y legible por máquina (ej.: `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | Una descripción legible de lo que salió mal. |
