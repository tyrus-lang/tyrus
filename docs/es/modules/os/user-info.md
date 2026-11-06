---
title: userInfo
description: Lee el login, directorio home y shell del usuario activo mediante el módulo nativo tyrus:os.
---

# userInfo

`userInfo` devuelve el login, el directorio home y el shell del usuario activo en una sola llamada, como un `UserInfo` nativo.

<SignatureBlock label="Firma">userInfo(): UserInfo</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-user-info.md -->
### Campos de UserInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `username` | `string` | El nombre de login (igual a [`userName()`](./user-name.md)). |
| `homedir` | `string` | El directorio home (igual a [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | El shell predeterminado del usuario. |
