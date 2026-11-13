---
title: Processos
description: Inicie programas externos e gerencie sua entrada, saída e status de encerramento.
---

# Processos

O módulo `tyrus:process` inicia programas externos sem shell. Cada processo filho expõe streams limitados e separados para entrada, saída e erro padrão.

## Importação

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";
```

`exec(command, args?)` passa o executável e cada argumento diretamente ao sistema operacional. Ele não interpreta aspas de shell, pipes, redirecionamentos ou outra sintaxe de shell.

## Referência da API

| Nome | Tipo | Descrição |
| --- | --- | --- |
| [`exec`](./exec.md) | Função | Inicia um processo filho e retorna `Result<ChildProcess, ProcessError>`. |
| [`ChildProcess`](./child-process.md) | Interface | Fornece o ID do processo, os streams padrão e os métodos `wait()` e `kill()`. |
| [`wait()`](./wait.md) | Método | Aguarda o encerramento do processo filho e retorna seu código de saída. |
| [`kill()`](./kill.md) | Método | Solicita o encerramento de um processo filho em execução. |
| [`ProcessError`](./errors.md) | Interface | Descreve falhas recuperáveis de `exec`, `wait()` e `kill()`. |
| [Processo atual](./current-process.md) | API ambiente | Documenta `process.argv`, as informações do processo e `process.exit()`. |

## Streams do Processo Filho

`stdin` é um `WritableStream<u8[]>`; feche-o com `await child.stdin.close()` para enviar EOF. `stdout` e `stderr` são valores independentes de `ReadableStream<u8[]>`. Os métodos desses streams estão documentados no módulo [`tyrus:stream`](../stream/).

Cada stream do processo usa um buffer limitado de 256 KiB e produz chunks de no máximo 64 KiB. Um consumidor lento pausa a leitura daquele descritor em vez de acumular a saída na memória. `stdout` e `stderr` preservam todos os bytes, inclusive bytes zero.

Liberar o último handle de `ChildProcess` encerra e coleta um filho que ainda está em execução. Um código de saída diferente de zero e bytes gravados em `stderr` são resultados normais do processo, não valores `ProcessError`.

::: warning Disponibilidade no playground
`tyrus:process` exige acesso a processos do sistema operacional e não está disponível em builds sandboxed.
:::
