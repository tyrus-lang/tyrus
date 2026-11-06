---
title: userInfo
description: Read the active user's login, home directory, and shell through the native tyrus:os module.
---

# userInfo

`userInfo` returns the active user's login, home directory, and shell in one call, as a native `UserInfo`.

<SignatureBlock label="Signature">userInfo(): UserInfo</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-user-info.md -->
### UserInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `username` | `string` | The login name (same as [`userName()`](./user-name.md)). |
| `homedir` | `string` | The home directory (same as [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | The user's default shell. |
