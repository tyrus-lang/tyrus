---
title: ChildProcess.wait
description: Wait for a child process to exit and read its exit result.
---

# ChildProcess.wait

`wait()` asynchronously waits until the operating system reports that the child has exited.

<SignatureBlock label="Signature">wait(): Promise&lt;Result&lt;i32, ProcessError&gt;&gt;</SignatureBlock>

## Result

The `Ok` value is the child's exit code, including non-zero codes. A non-zero exit is not a `ProcessError`; inspect the code and decide how the calling program should respond.

`Err` contains a `ProcessError` if the runtime cannot wait for the child. The stable error code is `ERR_PROCESS_WAIT_FAILED`.

## Usage

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";

async function run(): Promise<Result<i32, ProcessError>> {
    const child: ChildProcess = exec("tool", ["--version"])?;
    await child.stdin.close();
    return await child.wait();
}
```

Waiting does not consume or close `stdout` or `stderr`. Read or pipe those streams separately when the child produces output.
