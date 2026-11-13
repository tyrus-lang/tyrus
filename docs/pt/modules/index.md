---
title: Módulos nativos
description: APIs de runtime expostas por caminhos de import estáveis tyrus:*.
---

# Módulos nativos

Módulos nativos expõem funcionalidades de runtime por caminhos de import estáveis `tyrus:*`. Suas APIs são concisas, e suas implementações são linkadas em binários nativos. Todo módulo faz parte da distribuição padrão, sem package manager, sem npm install, sem runtime externo.

## Módulos disponíveis

| Módulo | Finalidade |
|--------|------------|
| [`tyrus:algorithm`](./algorithm/) | Estruturas de dados e algoritmos nativos para ordenação, busca, grafos, árvores e coleções. |
| [`tyrus:archive`](./archive/) | Cria, inspeciona e extrai com segurança arquivos ZIP, TAR e TAR.GZ. |
| [`tyrus:cli`](./cli/) | Helpers interativos de prompt de terminal. |
| [`tyrus:compress`](./compress/) | Comprime e descomprime bytes com Gzip, Deflate, Deflate cru e Brotli. |
| [`tyrus:crypto`](./crypto/) | Primitivas criptográficas: digest, HMAC, AES-256-GCM, Argon2, JWT e aleatório seguro. |
| [`tyrus:db`](./db/) | Snapshots versionados de schema SQLite, verificações somente leitura e contrato de erro do banco. |
| [`tyrus:encoding`](./encoding/) | Conversão explícita de UTF-8, UTF-16, hexadecimal, Base64, Base32 e radix. |
| [`tyrus:fs`](./fs/) | Leitura e escrita síncrona de arquivos para programas nativos. |
| [`tyrus:http`](./http/) | Servidor HTTP, rotas, resposta, status e helpers orientados a API. |
| [`tyrus:math`](./math/) | Constantes e funções matemáticas. |
| [`tyrus:memory`](./memory/) | Ownership explícito, apagamento seguro e consultas de layout em tempo de compilação. |
| [`tyrus:net`](./net/) | Parsing de endereços, resolução de nomes com `Result`, comunicação TCP e UDP assíncrona e sondagens condicionais à capacidade. |
| [`tyrus:os`](./os/) | Identificação multiplataforma do sistema operacional: plataforma, arquitetura, hostname, locale. |
| [`tyrus:process`](./process/) | Subprocessos sem shell com streams padrão de bytes limitados. |
| [`tyrus:simd`](./simd/) | Soma vetorizada de arrays `f32[]` e `f64[]` de mesmo tamanho, com erros recuperáveis em `Result`. |
| [`tyrus:stream`](./stream/) | Transporte progressivo limitado com capacidades separadas de leitura e escrita. |
| [`tyrus:task`](./task/) | Gerenciamento cooperativo de async tasks: cancelamento e suspensão temporizada. |
| [`tyrus:temporal`](./temporal/) | Instantes nativos, valores de calendário, durações e fusos horários IANA. |
| [`tyrus:test`](./test/) | Suítes de teste, asserções, hooks e modificadores executados por `tyrus test`. |
| [`tyrus:utils`](./utils/) | Helpers nativos de texto como estilo ANSI e banners FIGlet. |

## Estilo de import

<!-- @include: ../../.fragments/modules/import-style.md -->

Imports `tyrus:*` são resolvidos pelo compilador como módulos integrados.

## Usando módulos juntos

Módulos nativos se compõem naturalmente. Um projeto CLI típico pode ler config do filesystem, pedir input e aplicar estilos:

<!-- @include: ../../.fragments/modules/using-modules-together.md -->

## Fronteira de runtime

Módulos nativos devem documentar imports públicos, tipos de retorno, comportamento de erro e diagnósticos antes de serem tratados como estáveis. APIs que imprimem UI chrome de terminal usam stderr para manter stdout reservado à saída normal do programa.
