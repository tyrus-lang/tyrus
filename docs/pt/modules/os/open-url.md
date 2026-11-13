---
title: openUrl
description: Abra uma URL com o manipulador padrão da plataforma através do módulo nativo tyrus:os.
---

# openUrl

`openUrl` abre `url` com o manipulador padrão da plataforma (navegador, cliente de e-mail etc.), o mesmo efeito de clicar duas vezes num link.

<SignatureBlock label="Assinatura">openUrl(url: string): Result&lt;void, OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-open-url.md -->
Retorna `Err(OsError)` (`code: "ERR_OS_NO_HANDLER"`) se nenhum aplicativo está registrado para abrir `url`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador estável e legível por máquina (ex.: `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | Uma descrição legível do que deu errado. |
