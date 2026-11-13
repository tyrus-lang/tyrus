---
title: ChildProcess
description: Controle um processo filho e transporte seus streams padrão.
---

# ChildProcess

`ChildProcess` possui um programa externo iniciado e expõe seus descritores padrão como streams separados por capacidade.

```tyrus
interface ChildProcess {
    readonly pid: u32;
    readonly stdin: WritableStream<u8[]>;
    readonly stdout: ReadableStream<u8[]>;
    readonly stderr: ReadableStream<u8[]>;
    wait(): Promise<Result<i32, ProcessError>>;
    kill(): Promise<Result<void, ProcessError>>;
}
```

## Membros

| Membro | Assinatura | Finalidade |
| --- | --- | --- |
| `pid` | `u32` | Identificador do processo no sistema operacional. |
| `stdin` | `WritableStream<u8[]>` | Envia bytes para a entrada padrão. |
| `stdout` | `ReadableStream<u8[]>` | Recebe bytes da saída padrão. |
| `stderr` | `ReadableStream<u8[]>` | Recebe bytes do erro padrão. |
| [`wait()`](./wait.md) | `() => Promise<Result<i32, ProcessError>>` | Resolve com o código de saída ou um erro recuperável do processo. |
| [`kill()`](./kill.md) | `() => Promise<Result<void, ProcessError>>` | Solicita o encerramento e informa falhas recuperáveis. |

## Uso e erros

<!-- @include: ../../../.fragments/modules/process/child-process.md -->

Fechar `stdin` entrega EOF sem fechar a saída. Os streams de saída e erro permanecem legíveis após `kill()` para drenar bytes aceitos. Liberar o último handle encerra e coleta um filho ainda em execução.
