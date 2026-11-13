---
title: ChildProcess.kill
description: Solicite o encerramento de um processo filho em execução.
---

# ChildProcess.kill

`kill()` solicita de forma assíncrona ao sistema operacional que encerre o processo filho.

<SignatureBlock label="Assinatura">kill(): Promise&lt;Result&lt;void, ProcessError&gt;&gt;</SignatureBlock>

## Resultado

`Ok` significa que a solicitação de encerramento foi aceita. O resultado não contém o código de saída do processo filho; chame [`wait()`](./wait.md) para consultar o resultado final.

`Err` contém um `ProcessError` quando o handle do processo está fechado ou o sistema operacional não consegue encerrá-lo. Os códigos de erro estáveis são `ERR_PROCESS_CLOSED` e `ERR_PROCESS_KILL_FAILED`.

## Streams

Encerrar o processo filho não descarta bytes já aceitos pelos pipes de saída. Continue lendo `stdout` e `stderr` para drená-los e depois chame `wait()` para coletar o resultado de saída. Este método não fecha `stdin` implicitamente.

## Uso

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function stop(command: string): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec(command)?;
    await child.kill()?;
    return await child.wait();
}
```
