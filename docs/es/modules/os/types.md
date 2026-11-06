---
title: Interfaces y Enums
description: Enums e interfaces nativos de tyrus:os, OsPlatform, OsArch, NotifyUrgency, SystemCapability, UserInfo, CpuInfo, BatteryInfo, NetworkInterface, OsError, NotifyOptions, FontInfo y Color.
---

# Interfaces y Enums

`tyrus:os` exporta cuatro enums nativos y ocho interfaces nativas.

::: tip Las interfaces deben importarse por su nombre
A diferencia de los enums de abajo, `UserInfo`, `CpuInfo`, `BatteryInfo`, `NetworkInterface`, `OsError`, `NotifyOptions`, `FontInfo` y `Color` no están disponibles hasta que el archivo las importe explícitamente, `import { CpuInfo, cpus } from "tyrus:os";`, no solo `cpus`. Esto aplica incluso sin escribir el nombre del tipo en una anotación: `const cores = cpus(); cores[0].model` igual requiere que `CpuInfo` esté importado, ya que resolver `.model` necesita su lista de campos.
:::

## OsPlatform y OsArch

`OsPlatform` y `OsArch` son devueltos por [`platform()`](./platform.md) y [`arch()`](./arch.md). Ambos reflejan el objetivo para el cual se compiló este binario, no una suposición en tiempo de ejecución.

<!-- @include: ../../../.fragments/modules/os/types-example.md -->

### Miembros de OsPlatform

| Miembro | Significado |
|---------|-------------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Cualquier otro objetivo |

### Miembros de OsArch

| Miembro | Significado |
|---------|-------------|
| `X86_64` | x86 de 64 bits |
| `Aarch64` | ARM de 64 bits |
| `X86` | x86 de 32 bits |
| `Arm` | ARM de 32 bits |
| `Other` | Cualquier otro objetivo |

## NotifyUrgency

`NotifyUrgency` es el campo `urgency` de [`NotifyOptions`](#notifyoptions), pasado a [`notify()`](./notify.md).

### Miembros de NotifyUrgency

| Miembro | Significado |
|---------|-------------|
| `Low` | Prioridad baja, el entorno de escritorio puede no mostrarla en modo "no molestar". |
| `Normal` | Prioridad predeterminada. |
| `Critical` | Prioridad alta, algunos entornos de escritorio la mantienen en pantalla hasta que se descarta. |

## SystemCapability

`SystemCapability` es el tipo del argumento de [`hasCapability()`](./has-capability.md): cada miembro responde "¿se puede saber?"/"¿esto funcionaría?" para un rincón de `tyrus:os`, sin activar el efecto real.

### Miembros de SystemCapability

| Miembro | Significado |
|---------|-------------|
| `Notification` | Si una notificación de escritorio podría mostrarse plausiblemente. |
| `Clipboard` | Si el portapapeles del sistema se puede alcanzar. |
| `Battery` | Si esta máquina reporta al menos una batería. |
| `DarkMode` | Si la configuración de modo oscuro del escritorio es consultable. |
| `ScreenLock` | Si el estado del salvapantallas/pantalla de bloqueo es consultable (solo Linux). |
| `SessionLock` | Si el estado de bloqueo de sesión es consultable (solo Linux). |
| `SleepPrevention` | Si `preventSleep()`/`allowSleep()` realmente funcionarían. |
| `Fonts` | Si `systemFonts()` devolvería algo. |

## UserInfo

`UserInfo` es devuelto por [`userInfo()`](./user-info.md): el login, el directorio home y el shell del usuario activo en una sola llamada.

<!-- @include: ../../../.fragments/modules/os/user-info-example.md -->

### Campos de UserInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `username` | `string` | El nombre de login (igual a [`userName()`](./user-name.md)). |
| `homedir` | `string` | El directorio home (igual a [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | El shell predeterminado del usuario. |

## CpuInfo

`CpuInfo` es el tipo de los elementos del array devuelto por [`cpus()`](./cpus.md): una entrada por núcleo lógico de CPU.

<!-- @include: ../../../.fragments/modules/os/usage-cpus.md -->

### Campos de CpuInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `model` | `string` | El nombre del modelo de la CPU (ej.: `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | La frecuencia del núcleo, en MHz. |

## BatteryInfo

`BatteryInfo` es devuelto por [`battery()`](./battery.md).

<!-- @include: ../../../.fragments/modules/os/usage-battery.md -->

### Campos de BatteryInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `level` | `f32` | Estado de carga, de `0.0` a `1.0`. |
| `charging` | `boolean` | Si la batería está cargando en este momento. |
| `timeToFull` | `u32?` | Segundos hasta completar la carga, ausente a menos que esté cargando. |
| `timeToEmpty` | `u32?` | Segundos hasta agotarse, ausente a menos que esté descargando. |

## NetworkInterface

`NetworkInterface` es el tipo de los elementos del array devuelto en la variante `Ok` por [`networkInterfaces()`](./network-interfaces.md): una entrada por dirección IP local, así que un adaptador con dirección IPv4 e IPv6 aporta dos entradas.

<!-- @include: ../../../.fragments/modules/os/usage-network-interfaces.md -->

### Campos de NetworkInterface

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | El nombre del adaptador (ej.: `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | La dirección IP local. |
| `netmask` | `string` | La máscara de subred de `address`. |
| `family` | `string` | `"IPv4"` o `"IPv6"`. |

## OsError

`OsError` se devuelve en la variante `Err` por [`openUrl()`](./open-url.md), [`trash()`](./trash.md), [`preventSleep()`](./prevent-sleep.md), [`allowSleep()`](./allow-sleep.md) y [`networkInterfaces()`](./network-interfaces.md) cuando falla una operación, con un `code` estable y estructurado y un `message` legible.

`OsError` es un `Error`: un `catch (error)` sin anotación lo recibe, y `error is OsError` estrecha el binding a él, con todos los campos de `OsError`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Un identificador estable y legible por máquina (ej.: `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | Una descripción legible de lo que salió mal. |

## NotifyOptions

`NotifyOptions` es el tercer argumento opcional de [`notify()`](./notify.md). Todo campo es opcional; un campo omitido usa el valor predeterminado de la propia plataforma.

### Campos de NotifyOptions

| Campo | Tipo | Significado |
|-------|------|-------------|
| `icon` | `string?` | El nombre o ruta de un ícono para mostrar junto a la notificación. |
| `urgency` | `NotifyUrgency?` | Consulta [NotifyUrgency](#notifyurgency). El valor predeterminado es `Normal`. |
| `timeout` | `u32?` | Cuánto tiempo permanece visible la notificación, en milisegundos, antes de que el entorno de escritorio la descarte automáticamente. |

## FontInfo

`FontInfo` es el tipo del elemento del array devuelto por [`systemFonts()`](./system-fonts.md): una entrada por cada face de fuente, así que una familia con variantes regular/bold/italic/bold-italic contribuye con una entrada por variante.

<!-- @include: ../../../.fragments/modules/os/usage-system-fonts.md -->

### Campos de FontInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | El nombre de familia principal de la face. |
| `style` | `string` | `"normal"`, `"italic"`, o `"oblique"`. |
| `path` | `string` | La ruta del archivo de fuente, o una cadena vacía para una fuente incrustada en memoria en lugar de respaldada por un archivo. |

## Color

`Color` es el tipo del argumento de [`accessibleColorContrast()`](./accessible-color-contrast.md), construido como un object literal simple, de la misma forma que [`NotifyOptions`](#notifyoptions).

### Campos de Color

| Campo | Tipo | Significado |
|-------|------|-------------|
| `r` | `u8` | El canal rojo, de `0` a `255`. |
| `g` | `u8` | El canal verde, de `0` a `255`. |
| `b` | `u8` | El canal azul, de `0` a `255`. |
