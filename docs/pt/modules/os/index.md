---
title: tyrus:os
description: Identificação multiplataforma do sistema operacional baseada no runtime do tyrus.
---

# tyrus:os

`tyrus:os` expõe informações multiplataforma sobre o sistema operacional em que o binário compilado está rodando: plataforma, arquitetura, hostname, versão do kernel, locale, a constante de fim de linha, o usuário ativo e diretórios padrão. É somente leitura e puramente informativo, informações no nível do processo (`pid`, `cwd()`, `memoryUsage()`, etc.) ficam no global embutido `process`, documentado em [Variáveis de Ambiente](../../language/environment.md).

## Imports

<!-- @include: ../../../.fragments/modules/os/imports.md -->

## Identificação do Sistema

| Export | Assinatura | Finalidade |
|--------|------------|------------|
| [`platform`](./platform.md) | `() => OsPlatform` | A plataforma alvo. |
| [`arch`](./arch.md) | `() => OsArch` | A arquitetura de CPU alvo. |
| [`EOL`](./eol.md) | `string` | A constante nativa de fim de linha. |
| [`hostname`](./hostname.md) | `() => string` | O hostname de rede da máquina. |
| [`release`](./release.md) | `() => string` | A versão de release/build do kernel. |
| [`systemName`](./system-name.md) | `() => string` | O nome do sistema no padrão POSIX (`uname -s`). |
| [`locale`](./locale.md) | `() => string` | O código de locale atual. |
| [`language`](./language.md) | `() => string` | O subtag de idioma do locale atual. |

## Usuário e Diretórios

| Export | Assinatura | Finalidade |
|--------|------------|------------|
| [`userInfo`](./user-info.md) | `() => UserInfo` | Login, diretório home e shell numa única chamada. |
| [`userName`](./user-name.md) | `() => string` | O nome de login. |
| [`whoami`](./whoami.md) | `() => string?` | O nome real (GECOS), não o login. |
| [`homeDir`](./home-dir.md) | `() => string` | O diretório home do usuário. |
| [`cacheDir`](./cache-dir.md) | `() => string` | O diretório de cache do usuário. |
| [`configDir`](./config-dir.md) | `() => string` | O diretório de configuração do usuário. |
| [`dataDir`](./data-dir.md) | `() => string` | O diretório de dados do usuário. |
| [`tempDir`](./temp-dir.md) | `() => string` | O diretório temporário do sistema. |
| [`desktopDir`](./desktop-dir.md) | `() => string?` | O diretório da área de trabalho, se existir. |
| [`documentsDir`](./documents-dir.md) | `() => string?` | O diretório de documentos, se existir. |
| [`appDataDir`](./app-data-dir.md) | `(app: string) => string` | Um subdiretório específico de aplicação dentro de `dataDir`. |

## Hardware e Performance

| Export | Assinatura | Finalidade |
|--------|------------|------------|
| [`cpus`](./cpus.md) | `() => CpuInfo[]` | Uma entrada por núcleo lógico de CPU. |
| [`totalMem`](./total-mem.md) | `() => u64` | Memória total do sistema, em bytes. |
| [`freeMem`](./free-mem.md) | `() => u64` | Memória livre do sistema, em bytes. |
| [`uptime`](./uptime.md) | `() => u64` | Segundos desde a última inicialização do sistema. |
| [`loadAvg`](./load-avg.md) | `() => f64[]` | As médias de carga de 1/5/15 minutos. |
| [`battery`](./battery.md) | `() => BatteryInfo?` | Informações da primeira bateria, se houver. |
| [`cpuTemp`](./cpu-temp.md) | `(unit: string) => f32?` | A maior temperatura de sensor reportada. |

## Tela e Sessão

| Export | Assinatura | Propósito |
|--------|-----------|---------|
| [`isDarkMode`](./is-dark-mode.md) | `() => boolean` | Se a área de trabalho está em modo escuro. |
| [`terminalSize`](./terminal-size.md) | `() => [u16, u16]?` | As colunas e linhas do terminal. |
| [`isScreenLocked`](./is-screen-locked.md) | `() => boolean` | Se a proteção de tela/tela de bloqueio está ativa. |
| [`isSessionLocked`](./is-session-locked.md) | `() => boolean` | Se a sessão do usuário atual está bloqueada. |
| [`preventSleep`](./prevent-sleep.md) | `() => Result<void, OsError>` | Inibe a suspensão automática do sistema/tela. |
| [`allowSleep`](./allow-sleep.md) | `() => Result<void, OsError>` | Reverte `preventSleep`. |

`isScreenLocked`/`isSessionLocked` são de melhor esforço e, por enquanto, só Linux, `false` em outras plataformas, ou quando o serviço subjacente não pode ser alcançado.

## Fontes e Acessibilidade

| Export | Assinatura | Propósito |
|--------|-----------|---------|
| [`systemFonts`](./system-fonts.md) | `() => FontInfo[]` | Uma entrada por face de fonte instalada no sistema. |
| [`accessibleColorContrast`](./accessible-color-contrast.md) | `(fg: Color, bg: Color) => boolean` | Se um par de cores atende à proporção de contraste AA do WCAG. |

## Utilitários

| Export | Assinatura | Propósito |
|--------|-----------|---------|
| [`hasCapability`](./has-capability.md) | `(cap: SystemCapability) => boolean` | Se uma dada capacidade está disponível neste sistema. |

Veja [Interfaces e Enums](./types.md) para o formato de `OsPlatform`, `OsArch`, `SystemCapability`, `UserInfo`, `CpuInfo`, `BatteryInfo`, `FontInfo`, e `Color`.

::: tip Valores não resolvidos nunca abortam o programa
`hostname()`, `release()`, `systemName()`, `userName()`, e toda função de diretório caem para `"unknown"` ou uma string vazia se o sistema operacional não reportar um valor. `locale()` e `language()` caem para `"und"`, o subtag padrão de "locale indeterminado". `whoami()`, `desktopDir()`, `documentsDir()`, `battery()`, e `cpuTemp()` devolvem um tipo opcional porque ausência é um resultado normal (um sistema headless, uma conta sem nome real configurado, um desktop sem bateria, um container sem sensor térmico), nenhuma dessas funções pode falhar o programa como um erro de filesystem falharia. A única exceção é o argumento `unit` de `cpuTemp`: `"celsius"`/`"fahrenheit"` são os únicos valores válidos, e qualquer outro aborta o programa.
:::

## Exemplo

<!-- @include: ../../../.fragments/modules/os/example.md -->
