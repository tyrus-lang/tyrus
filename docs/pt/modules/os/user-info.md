---
title: userInfo
description: Leia o login, diretório home e shell do usuário ativo através do módulo nativo tyrus:os.
---

# userInfo

`userInfo` devolve o login, o diretório home e o shell do usuário ativo numa única chamada, como um `UserInfo` nativo.

<SignatureBlock label="Assinatura">userInfo(): UserInfo</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-user-info.md -->
### Campos de UserInfo

| Campo | Tipo | Significado |
|-------|------|--------------|
| `username` | `string` | O nome de login (igual a [`userName()`](./user-name.md)). |
| `homedir` | `string` | O diretório home (igual a [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | O shell padrão do usuário. |
