---
title: Interfaces and Enums
description: Native enums and interfaces from tyrus:os, OsPlatform, OsArch, NotifyUrgency, SystemCapability, UserInfo, CpuInfo, BatteryInfo, NetworkInterface, OsError, NotifyOptions, FontInfo, and Color.
---

# Interfaces and Enums

`tyrus:os` exports four native enums and eight native interfaces.

::: tip Interfaces must be imported by name
Unlike the enums below, `UserInfo`, `CpuInfo`, `BatteryInfo`, `NetworkInterface`, `OsError`, `NotifyOptions`, `FontInfo`, and `Color` are not available until the file imports them explicitly, `import { CpuInfo, cpus } from "tyrus:os";`, not just `cpus`. This applies even without writing the type name in an annotation: `const cores = cpus(); cores[0].model` still requires `CpuInfo` to be imported, since resolving `.model` needs its field list.
:::

## OsPlatform and OsArch

`OsPlatform` and `OsArch` are returned by [`platform()`](./platform.md) and [`arch()`](./arch.md). Both reflect the target this binary was compiled for, not a runtime guess.

<!-- @include: ../../.fragments/modules/os/types-example.md -->

### OsPlatform Members

| Member | Meaning |
|--------|---------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Any other target |

### OsArch Members

| Member | Meaning |
|--------|---------|
| `X86_64` | 64-bit x86 |
| `Aarch64` | 64-bit ARM |
| `X86` | 32-bit x86 |
| `Arm` | 32-bit ARM |
| `Other` | Any other target |

## NotifyUrgency

`NotifyUrgency` is [`NotifyOptions`](#notifyoptions)'s `urgency` field, passed to [`notify()`](./notify.md).

### NotifyUrgency Members

| Member | Meaning |
|--------|---------|
| `Low` | Low priority, the desktop environment may not show it at all in "do not disturb" mode. |
| `Normal` | Default priority. |
| `Critical` | High priority, some desktop environments keep it on screen until dismissed. |

## SystemCapability

`SystemCapability` is [`hasCapability()`](./has-capability.md)'s argument type: each member answers "can I find out?"/"would this work?" for one corner of `tyrus:os`, without triggering the real effect.

### SystemCapability Members

| Member | Meaning |
|--------|---------|
| `Notification` | Whether a desktop notification could plausibly be shown. |
| `Clipboard` | Whether the system clipboard can be reached. |
| `Battery` | Whether this machine reports at least one battery. |
| `DarkMode` | Whether the desktop's dark-mode setting is queryable at all. |
| `ScreenLock` | Whether the screensaver/lock-screen state is queryable at all (Linux only). |
| `SessionLock` | Whether the session-lock state is queryable at all (Linux only). |
| `SleepPrevention` | Whether `preventSleep()`/`allowSleep()` would actually work. |
| `Fonts` | Whether `systemFonts()` would return anything. |

## UserInfo

`UserInfo` is returned by [`userInfo()`](./user-info.md): the active user's login, home directory, and shell in one call.

<!-- @include: ../../.fragments/modules/os/user-info-example.md -->

### UserInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `username` | `string` | The login name (same as [`userName()`](./user-name.md)). |
| `homedir` | `string` | The home directory (same as [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | The user's default shell. |

## CpuInfo

`CpuInfo` is the element type of the array returned by [`cpus()`](./cpus.md): one entry per logical CPU core.

<!-- @include: ../../.fragments/modules/os/usage-cpus.md -->

### CpuInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `model` | `string` | The CPU model name (e.g. `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | The core's frequency, in MHz. |

## BatteryInfo

`BatteryInfo` is returned by [`battery()`](./battery.md).

<!-- @include: ../../.fragments/modules/os/usage-battery.md -->

### BatteryInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `level` | `f32` | State of charge, from `0.0` to `1.0`. |
| `charging` | `boolean` | Whether the battery is currently charging. |
| `timeToFull` | `u32?` | Seconds until full, absent unless currently charging. |
| `timeToEmpty` | `u32?` | Seconds until empty, absent unless currently discharging. |

## NetworkInterface

`NetworkInterface` is the element type of the array returned in the `Ok` variant by [`networkInterfaces()`](./network-interfaces.md): one entry per local IP address, so an adapter with both an IPv4 and an IPv6 address contributes two entries.

<!-- @include: ../../.fragments/modules/os/usage-network-interfaces.md -->

### NetworkInterface Fields

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The adapter's name (e.g. `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | The local IP address. |
| `netmask` | `string` | The subnet mask for `address`. |
| `family` | `string` | Either `"IPv4"` or `"IPv6"`. |

## OsError

`OsError` is returned in the `Err` variant by [`openUrl()`](./open-url.md), [`trash()`](./trash.md), [`preventSleep()`](./prevent-sleep.md), [`allowSleep()`](./allow-sleep.md) and [`networkInterfaces()`](./network-interfaces.md) when an operation fails, with a stable, structured `code` and a human-readable `message`.

`OsError` is an `Error`: a `catch (error)` without an annotation receives it, and `error is OsError` narrows the binding to it, with every field of `OsError`.

### OsError Fields

| Field | Type | Meaning |
|-------|------|---------|
| `code` | `string` | A stable, machine-readable identifier (e.g. `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | A human-readable description of what went wrong. |

## NotifyOptions

`NotifyOptions` is [`notify()`](./notify.md)'s optional third argument. Every field is optional; an omitted field uses the platform's own default.

### NotifyOptions Fields

| Field | Type | Meaning |
|-------|------|---------|
| `icon` | `string?` | The name or path of an icon to show alongside the notification. |
| `urgency` | `NotifyUrgency?` | See [NotifyUrgency](#notifyurgency). Defaults to `Normal`. |
| `timeout` | `u32?` | How long the notification stays visible, in milliseconds, before the desktop environment dismisses it automatically. |

## FontInfo

`FontInfo` is the element type of the array returned by [`systemFonts()`](./system-fonts.md): one entry per font face, so a family with regular/bold/italic/bold-italic variants contributes one entry per variant.

<!-- @include: ../../.fragments/modules/os/usage-system-fonts.md -->

### FontInfo Fields

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The face's primary family name. |
| `style` | `string` | Either `"normal"`, `"italic"`, or `"oblique"`. |
| `path` | `string` | The font file's path, or an empty string for a font embedded in memory rather than backed by a file. |

## Color

`Color` is [`accessibleColorContrast()`](./accessible-color-contrast.md)'s argument type, built as a plain object literal, the same way [`NotifyOptions`](#notifyoptions) is.

### Color Fields

| Field | Type | Meaning |
|-------|------|---------|
| `r` | `u8` | The red channel, `0` to `255`. |
| `g` | `u8` | The green channel, `0` to `255`. |
| `b` | `u8` | The blue channel, `0` to `255`. |
