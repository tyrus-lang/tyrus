---
title: ChildProcess.wait
description: Aguarde o processo filho encerrar e leia o resultado da saída.
---

# ChildProcess.wait

`wait()` aguarda de forma assíncrona até que o sistema operacional informe que o processo filho encerrou.

<SignatureBlock label="Assinatura">wait(): Promise&lt;Result&lt;i32, ProcessError&gt;&gt;</SignatureBlock>

## Resultado

O valor `Ok` é o código de saída do processo filho, inclusive códigos diferentes de zero. Uma saída diferente de zero não é um `ProcessError`; verifique o código e decida como o programa chamador deve responder.

`Err` contém um `ProcessError` se o runtime não conseguir aguardar o processo filho. O código de erro estável é `ERR_PROCESS_WAIT_FAILED`.

## Uso

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function run(): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec("tool", ["--version"])?;
    await child.stdin.close();
    return await child.wait();
}
```

Aguardar não consome nem fecha `stdout` ou `stderr`. Leia ou encaminhe esses streams separadamente quando o processo filho produzir saída.
