---
title: tyrus:os
description: Cross-platform operating system identification backed by the tyrus runtime.
---

# tyrus:os

`tyrus:os` exposes cross-platform information about the operating system the compiled binary is running on: platform, architecture, hostname, kernel release, locale, the line-ending constant, the active user, and standard directories. It is read-only and purely informational, process-level information (`pid`, `cwd()`, `memoryUsage()`, etc.) lives on the built-in `process` global instead, documented in [Environment Variables](../../language/environment.md#process-information).

## Imports

<!-- @include: ../../.fragments/modules/os/imports.md -->

## System Identification

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`platform`](./platform.md) | `() => OsPlatform` | The target platform. |
| [`arch`](./arch.md) | `() => OsArch` | The target CPU architecture. |
| [`EOL`](./eol.md) | `string` | The native line-ending constant. |
| [`hostname`](./hostname.md) | `() => string` | The machine's network hostname. |
| [`release`](./release.md) | `() => string` | The kernel release/build version. |
| [`systemName`](./system-name.md) | `() => string` | The POSIX-style (`uname -s`) system name. |
| [`locale`](./locale.md) | `() => string` | The current locale tag. |
| [`language`](./language.md) | `() => string` | The current locale's language subtag. |

## User and Directories

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`userInfo`](./user-info.md) | `() => UserInfo` | The login, home directory, and shell in one call. |
| [`userName`](./user-name.md) | `() => string` | The login name. |
| [`whoami`](./whoami.md) | `() => string?` | The real name (GECOS), not the login. |
| [`homeDir`](./home-dir.md) | `() => string` | The user's home directory. |
| [`cacheDir`](./cache-dir.md) | `() => string` | The user's cache directory. |
| [`configDir`](./config-dir.md) | `() => string` | The user's config directory. |
| [`dataDir`](./data-dir.md) | `() => string` | The user's data directory. |
| [`tempDir`](./temp-dir.md) | `() => string` | The system's temporary directory. |
| [`desktopDir`](./desktop-dir.md) | `() => string?` | The user's desktop directory, if any. |
| [`documentsDir`](./documents-dir.md) | `() => string?` | The user's documents directory, if any. |
| [`appDataDir`](./app-data-dir.md) | `(app: string) => string` | An application-specific subdirectory of `dataDir`. |

## Hardware and Performance

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`cpus`](./cpus.md) | `() => CpuInfo[]` | One entry per logical CPU core. |
| [`totalMem`](./total-mem.md) | `() => u64` | Total system memory, in bytes. |
| [`freeMem`](./free-mem.md) | `() => u64` | Free system memory, in bytes. |
| [`uptime`](./uptime.md) | `() => u64` | Seconds since the system itself last booted. |
| [`loadAvg`](./load-avg.md) | `() => f64[]` | The 1/5/15-minute load averages. |
| [`battery`](./battery.md) | `() => BatteryInfo?` | The first battery's info, if any. |
| [`cpuTemp`](./cpu-temp.md) | `(unit: string) => f32?` | The highest reported sensor temperature. |

## Screen and Session

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`isDarkMode`](./is-dark-mode.md) | `() => boolean` | Whether the desktop is in dark mode. |
| [`terminalSize`](./terminal-size.md) | `() => [u16, u16]?` | The terminal's columns and rows. |
| [`isScreenLocked`](./is-screen-locked.md) | `() => boolean` | Whether the screensaver/lock screen is active. |
| [`isSessionLocked`](./is-session-locked.md) | `() => boolean` | Whether the current user session is locked. |
| [`preventSleep`](./prevent-sleep.md) | `() => Result<void, OsError>` | Inhibit automatic system/display sleep. |
| [`allowSleep`](./allow-sleep.md) | `() => Result<void, OsError>` | Reverse `preventSleep`. |

`isScreenLocked`/`isSessionLocked` are best effort and Linux-only for now, `false` on other platforms, or when the underlying service cannot be reached.

## Fonts and Accessibility

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`systemFonts`](./system-fonts.md) | `() => FontInfo[]` | One entry per font face installed on the system. |
| [`accessibleColorContrast`](./accessible-color-contrast.md) | `(fg: Color, bg: Color) => boolean` | Whether a color pair meets the WCAG AA contrast ratio. |

## Utilities

| Export | Signature | Purpose |
|--------|-----------|---------|
| [`hasCapability`](./has-capability.md) | `(cap: SystemCapability) => boolean` | Whether a given capability is available on this system. |

See [Interfaces and Enums](./types.md) for the shape of `OsPlatform`, `OsArch`, `SystemCapability`, `UserInfo`, `CpuInfo`, `BatteryInfo`, `FontInfo`, and `Color`.

::: tip Unresolved values never abort the program
`hostname()`, `release()`, `systemName()`, `userName()`, and every directory function fall back to `"unknown"` or an empty string if the operating system does not report a value. `locale()` and `language()` fall back to `"und"`, the standard "undetermined locale" subtag. `whoami()`, `desktopDir()`, `documentsDir()`, `battery()`, and `cpuTemp()` return an optional type because absence is a normal outcome (a headless system, an account with no real name set, a desktop with no battery, a container with no thermal sensor), none of these functions can fail the program the way a filesystem error does. The one exception is `cpuTemp`'s `unit` argument: `"celsius"`/`"fahrenheit"` are the only valid values, and anything else aborts the program.
:::

## Example

<!-- @include: ../../.fragments/modules/os/example.md -->
