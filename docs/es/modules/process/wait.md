---
title: ChildProcess.wait
description: Espera a que termine el proceso hijo y consulta el resultado de salida.
---

# ChildProcess.wait

`wait()` espera de forma asíncrona hasta que el sistema operativo informe que el proceso hijo terminó.

<SignatureBlock label="Firma">wait(): Promise&lt;Result&lt;i32, ProcessError&gt;&gt;</SignatureBlock>

## Resultado

El valor `Ok` es el código de salida del proceso hijo, incluidos los códigos distintos de cero. Una salida distinta de cero no es un `ProcessError`; consulta el código y decide cómo debe responder el programa que llama.

`Err` contiene un `ProcessError` si el runtime no puede esperar al proceso hijo. El código de error estable es `ERR_PROCESS_WAIT_FAILED`.

## Uso

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function run(): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec("tool", ["--version"])?;
    await child.stdin.close();
    return await child.wait();
}
```

Esperar no consume ni cierra `stdout` o `stderr`. Lee o canaliza esos streams por separado cuando el proceso hijo produzca salida.
