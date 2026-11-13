---
title: ChildProcess.kill
description: Request termination of a running child process.
---

# ChildProcess.kill

`kill()` asynchronously asks the operating system to terminate the child process.

<SignatureBlock label="Signature">kill(): Promise&lt;Result&lt;void, ProcessError&gt;&gt;</SignatureBlock>

## Result

`Ok` means the termination request was accepted. It does not contain the child's exit code; call [`wait()`](./wait.md) to observe the final exit result.

`Err` contains a `ProcessError` when the process handle is closed or the operating system cannot terminate the child. The stable error codes are `ERR_PROCESS_CLOSED` and `ERR_PROCESS_KILL_FAILED`.

## Streams

Killing the child does not discard bytes already accepted by its output pipes. Continue reading `stdout` and `stderr` to drain them, then call `wait()` to collect the exit result. The child's `stdin` is not implicitly closed by this method.

## Usage

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function stop(command: string): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec(command)?;
    await child.kill()?;
    return await child.wait();
}
```
