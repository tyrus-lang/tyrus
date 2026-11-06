---
title: ChildProcess.kill
description: Solicita la terminación de un proceso hijo en ejecución.
---

# ChildProcess.kill

`kill()` solicita de forma asíncrona al sistema operativo que termine el proceso hijo.

<SignatureBlock label="Firma">kill(): Promise&lt;Result&lt;void, ProcessError&gt;&gt;</SignatureBlock>

## Resultado

`Ok` significa que se aceptó la solicitud de terminación. El resultado no contiene el código de salida del proceso hijo; llama a [`wait()`](./wait.md) para consultar el resultado final.

`Err` contiene un `ProcessError` cuando el handle del proceso está cerrado o el sistema operativo no puede terminarlo. Los códigos de error estables son `ERR_PROCESS_CLOSED` y `ERR_PROCESS_KILL_FAILED`.

## Streams

Terminar el proceso hijo no descarta los bytes ya aceptados por los pipes de salida. Sigue leyendo `stdout` y `stderr` para drenarlos y luego llama a `wait()` para obtener el resultado de salida. Este método no cierra `stdin` de forma implícita.

## Uso

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function stop(command: string): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec(command)?;
    await child.kill()?;
    return await child.wait();
}
```
