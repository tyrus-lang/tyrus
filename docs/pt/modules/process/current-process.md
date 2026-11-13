---
title: Processo atual
description: Leia os argumentos e as informações do processo ou defina o código de saída do programa.
---

# Processo atual

O namespace ambiente `process` descreve o programa Tyrus em execução. Esses valores estão disponíveis sem importar `tyrus:process`; esse módulo serve para iniciar e gerenciar processos filhos.

## Referência da API

| Membro | Tipo ou assinatura | Descrição |
| --- | --- | --- |
| `process.argv` | `string[]` | Argumentos de linha de comando. O índice `0` é o caminho do executável; os argumentos do usuário começam no índice `1`. |
| `process.pid` | `u32` | Identificador do processo do programa em execução. |
| `process.ppid` | `u32` | Identificador do processo pai. |
| `process.cwd()` | `() => string` | Retorna o diretório de trabalho atual. |
| `process.uptime()` | `() => u64` | Retorna o tempo de execução decorrido em milissegundos. |
| `process.memoryUsage()` | `() => MemoryInfo` | Retorna o uso de memória residente em bytes. |
| `process.cpuUsage()` | `() => CpuUsage` | Retorna o tempo de CPU acumulado do usuário e do sistema em microssegundos. |
| `process.exit(code)` | `(code: i32) => never` | Encerra o programa com o código informado. |

## Argumentos de Linha de Comando

Argumentos depois de `--` em `tyrus run` são passados ao programa, e não ao compilador. `Array.at()` retorna `string?`, então trate explicitamente um argumento que pode estar ausente.

<!-- @include: ../../../.fragments/docs/environment/argv.md -->

Como um programa Tyrus compilado é o próprio executável, o primeiro argumento do usuário fica no índice `1`. Código portado de um runtime em que o índice `0` é um interpretador e o índice `1` é o script deve usar `slice(1)`, e não `slice(2)`.

## Informações do Processo

`process.memoryUsage()` retorna este valor:

```tyrus
interface MemoryInfo {
    readonly rss: u64;
}
```

`rss` é o tamanho residente do processo em bytes.

`process.cpuUsage()` retorna este valor:

```tyrus
interface CpuUsage {
    readonly user: u64;
    readonly system: u64;
}
```

`user` e `system` são o tempo de CPU acumulado em microssegundos desde o início do processo, não porcentagens. Alvos Unix usam `getrusage`; no Windows, os contadores de 100 nanossegundos de `GetProcessTimes` são convertidos em microssegundos. Outros alvos retornam `0` nos dois campos.

<!-- @include: ../../../.fragments/docs/environment/process-info.md -->

`process.exit(code)` encerra o programa imediatamente, então statements depois dele são inalcançáveis. Um programa que retorna normalmente sai com o código `0`.
