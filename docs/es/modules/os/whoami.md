---
title: whoami
description: Lee el nombre real del usuario activo mediante el módulo nativo tyrus:os.
---

# whoami

`whoami` devuelve el nombre real (no el login) del dueño del proceso, ej.: `"Ada Lovelace"` en lugar de `"ada"`.

<SignatureBlock label="Firma">whoami(): string?</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-whoami.md -->
::: tip La ausencia es un resultado válido
Devuelve ausencia (no un programa abortado) si el sistema operativo no tiene un nombre real configurado para la cuenta, ej.: sin campo GECOS, o una plataforma donde el nombre real no es un concepto distinto del login.
:::

Consulta también [`userName`](./user-name.md) y [`userInfo`](./user-info.md), que devuelven el nombre de login.
