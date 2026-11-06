---
title: ChildProcess
description: Control a child process and transport its standard streams.
---

# ChildProcess

`ChildProcess` owns a started external program and exposes its standard descriptors as capability-separated streams.

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

## Members

| Member | Signature | Purpose |
| --- | --- | --- |
| `pid` | `u32` | Operating-system process identifier. |
| `stdin` | `WritableStream<u8[]>` | Sends bytes to standard input. |
| `stdout` | `ReadableStream<u8[]>` | Receives standard output bytes. |
| `stderr` | `ReadableStream<u8[]>` | Receives standard error bytes. |
| [`wait()`](./wait.md) | `() => Promise<Result<i32, ProcessError>>` | Resolves with the exit code or a recoverable process error. |
| [`kill()`](./kill.md) | `() => Promise<Result<void, ProcessError>>` | Requests termination of a running child and reports recoverable failures. |

## Usage and errors

<!-- @include: ../../.fragments/modules/process/child-process.md -->

Closing `stdin` delivers EOF without closing output. Output and error streams remain independently readable after `kill()` so accepted bytes can drain. Releasing the last handle terminates and reaps a child that is still running.
