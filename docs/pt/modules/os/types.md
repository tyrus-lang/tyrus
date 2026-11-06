---
title: Interfaces e Enums
description: Enums e interfaces nativos de tyrus:os, OsPlatform, OsArch, NotifyUrgency, SystemCapability, UserInfo, CpuInfo, BatteryInfo, NetworkInterface, OsError, NotifyOptions, FontInfo e Color.
---

# Interfaces e Enums

`tyrus:os` exporta quatro enums nativos e oito interfaces nativas.

::: tip Interfaces precisam ser importadas pelo nome
Diferente dos enums abaixo, `UserInfo`, `CpuInfo`, `BatteryInfo`, `NetworkInterface`, `OsError`, `NotifyOptions`, `FontInfo` e `Color` não ficam disponíveis até o arquivo importá-las explicitamente, `import { CpuInfo, cpus } from "tyrus:os";`, não só `cpus`. Isso vale mesmo sem escrever o nome do tipo numa anotação: `const cores = cpus(); cores[0].model` ainda exige `CpuInfo` importado, já que resolver `.model` precisa da lista de campos dela.
:::

## OsPlatform e OsArch

`OsPlatform` e `OsArch` são devolvidos por [`platform()`](./platform.md) e [`arch()`](./arch.md). Ambos refletem o alvo para o qual este binário foi compilado, não um palpite em tempo de execução.

<!-- @include: ../../../.fragments/modules/os/types-example.md -->

### Membros de OsPlatform

| Membro | Significado |
|--------|-------------|
| `Linux` | Linux |
| `MacOS` | macOS |
| `Windows` | Windows |
| `Other` | Qualquer outro alvo |

### Membros de OsArch

| Membro | Significado |
|--------|-------------|
| `X86_64` | x86 64-bit |
| `Aarch64` | ARM 64-bit |
| `X86` | x86 32-bit |
| `Arm` | ARM 32-bit |
| `Other` | Qualquer outro alvo |

## NotifyUrgency

`NotifyUrgency` é o campo `urgency` de [`NotifyOptions`](#notifyoptions), passado para [`notify()`](./notify.md).

### Membros de NotifyUrgency

| Membro | Significado |
|--------|-------------|
| `Low` | Prioridade baixa, o ambiente de desktop pode nem mostrar em modo "não perturbe". |
| `Normal` | Prioridade padrão. |
| `Critical` | Prioridade alta, alguns ambientes de desktop mantêm na tela até ser dispensada. |

## SystemCapability

`SystemCapability` é o tipo do argumento de [`hasCapability()`](./has-capability.md): cada membro responde "dá pra descobrir?"/"isso funcionaria?" para um canto de `tyrus:os`, sem disparar o efeito real.

### Membros de SystemCapability

| Membro | Significado |
|--------|-------------|
| `Notification` | Se uma notificação de desktop poderia plausivelmente ser mostrada. |
| `Clipboard` | Se a área de transferência do sistema pode ser alcançada. |
| `Battery` | Se esta máquina reporta pelo menos uma bateria. |
| `DarkMode` | Se a configuração de modo escuro do desktop é consultável. |
| `ScreenLock` | Se o estado de proteção de tela/tela de bloqueio é consultável (só Linux). |
| `SessionLock` | Se o estado de bloqueio de sessão é consultável (só Linux). |
| `SleepPrevention` | Se `preventSleep()`/`allowSleep()` realmente funcionariam. |
| `Fonts` | Se `systemFonts()` retornaria alguma coisa. |

## UserInfo

`UserInfo` é devolvido por [`userInfo()`](./user-info.md): o login, o diretório home e o shell do usuário ativo numa única chamada.

<!-- @include: ../../../.fragments/modules/os/user-info-example.md -->

### Campos de UserInfo

| Campo | Tipo | Significado |
|-------|------|--------------|
| `username` | `string` | O nome de login (igual a [`userName()`](./user-name.md)). |
| `homedir` | `string` | O diretório home (igual a [`homeDir()`](./home-dir.md)). |
| `shell` | `string` | O shell padrão do usuário. |

## CpuInfo

`CpuInfo` é o tipo dos elementos do array devolvido por [`cpus()`](./cpus.md): uma entrada por núcleo lógico de CPU.

<!-- @include: ../../../.fragments/modules/os/usage-cpus.md -->

### Campos de CpuInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `model` | `string` | O nome do modelo da CPU (ex.: `"Intel(R) Core(TM) i5-10400F CPU @ 2.90GHz"`). |
| `speed` | `u32` | A frequência do núcleo, em MHz. |

## BatteryInfo

`BatteryInfo` é devolvido por [`battery()`](./battery.md).

<!-- @include: ../../../.fragments/modules/os/usage-battery.md -->

### Campos de BatteryInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `level` | `f32` | Estado de carga, de `0.0` a `1.0`. |
| `charging` | `boolean` | Se a bateria está carregando no momento. |
| `timeToFull` | `u32?` | Segundos até completar a carga, ausente a menos que esteja carregando. |
| `timeToEmpty` | `u32?` | Segundos até esgotar, ausente a menos que esteja descarregando. |

## NetworkInterface

`NetworkInterface` é o tipo dos elementos do array devolvido na variante `Ok` por [`networkInterfaces()`](./network-interfaces.md): uma entrada por endereço IP local, então uma placa com endereço IPv4 e IPv6 contribui com duas entradas.

<!-- @include: ../../../.fragments/modules/os/usage-network-interfaces.md -->

### Campos de NetworkInterface

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | O nome da placa (ex.: `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | O endereço IP local. |
| `netmask` | `string` | A máscara de sub-rede de `address`. |
| `family` | `string` | `"IPv4"` ou `"IPv6"`. |

## OsError

`OsError` é retornado na variante `Err` por [`openUrl()`](./open-url.md), [`trash()`](./trash.md), [`preventSleep()`](./prevent-sleep.md), [`allowSleep()`](./allow-sleep.md) e [`networkInterfaces()`](./network-interfaces.md) quando uma operação falha, com um `code` estável e estruturado e uma `message` legível.

`OsError` é um `Error`: um `catch (error)` sem anotação o recebe, e `error is OsError` estreita o binding para ele, com todos os campos de `OsError`.

### Campos de OsError

| Campo | Tipo | Significado |
|-------|------|-------------|
| `code` | `string` | Um identificador estável e legível por máquina (ex.: `"ERR_OS_NO_HANDLER"`). |
| `message` | `string` | Uma descrição legível do que deu errado. |

## NotifyOptions

`NotifyOptions` é o terceiro argumento opcional de [`notify()`](./notify.md). Todo campo é opcional; um campo omitido usa o padrão da própria plataforma.

### Campos de NotifyOptions

| Campo | Tipo | Significado |
|-------|------|-------------|
| `icon` | `string?` | O nome ou caminho de um ícone para mostrar junto da notificação. |
| `urgency` | `NotifyUrgency?` | Veja [NotifyUrgency](#notifyurgency). Padrão é `Normal`. |
| `timeout` | `u32?` | Por quanto tempo a notificação fica visível, em milissegundos, antes do ambiente de desktop dispensá-la automaticamente. |

## FontInfo

`FontInfo` é o tipo do elemento do array retornado por [`systemFonts()`](./system-fonts.md): uma entrada por face de fonte, então uma família com variantes regular/bold/italic/bold-italic contribui com uma entrada por variante.

<!-- @include: ../../../.fragments/modules/os/usage-system-fonts.md -->

### Campos de FontInfo

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | O nome de família primário da face. |
| `style` | `string` | `"normal"`, `"italic"`, ou `"oblique"`. |
| `path` | `string` | O caminho do arquivo de fonte, ou uma string vazia para uma fonte embutida em memória em vez de apoiada por um arquivo. |

## Color

`Color` é o tipo do argumento de [`accessibleColorContrast()`](./accessible-color-contrast.md), construído como um object literal simples, do mesmo jeito que [`NotifyOptions`](#notifyoptions).

### Campos de Color

| Campo | Tipo | Significado |
|-------|------|-------------|
| `r` | `u8` | O canal vermelho, de `0` a `255`. |
| `g` | `u8` | O canal verde, de `0` a `255`. |
| `b` | `u8` | O canal azul, de `0` a `255`. |
