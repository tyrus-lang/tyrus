---
title: Processes
description: Start external programs and manage their input, output, and exit status.
---

# Processes

The `tyrus:process` module starts external programs without a shell. Each child exposes separate bounded streams for standard input, output, and error.

## Import

```tyrus
import { ChildProcess, ProcessError, exec } from "tyrus:process";
```

`exec(command, args?)` passes the executable and each argument directly to the operating system. It does not parse shell quoting, pipes, redirects, or other shell syntax.

## API Reference

| Name | Kind | Description |
| --- | --- | --- |
| [`exec`](./exec.md) | Function | Starts a child process and returns `Result<ChildProcess, ProcessError>`. |
| [`ChildProcess`](./child-process.md) | Interface | Provides the process ID and standard streams, plus `wait()` and `kill()`. |
| [`wait()`](./wait.md) | Method | Waits for the child to exit and returns its exit code. |
| [`kill()`](./kill.md) | Method | Requests termination of a running child. |
| [`ProcessError`](./errors.md) | Interface | Describes recoverable failures from `exec`, `wait()`, and `kill()`. |
| [Current process](./current-process.md) | Ambient API | Documents `process.argv`, process information, and `process.exit()`. |

## Child Process Streams

`stdin` is a `WritableStream<u8[]>`; close it with `await child.stdin.close()` to send EOF. `stdout` and `stderr` are independent `ReadableStream<u8[]>` values. These stream methods are documented in the [`tyrus:stream` module](../stream/).

Each process stream uses a 256 KiB bounded buffer and produces chunks of at most 64 KiB. A slow consumer pauses reads from that descriptor instead of accumulating output in memory. `stdout` and `stderr` preserve all bytes, including zero bytes.

Releasing the final `ChildProcess` handle terminates and reaps a child that is still running. A non-zero exit code and bytes written to `stderr` are normal process results, not `ProcessError` values.

::: warning Playground availability
`tyrus:process` requires operating-system process access and is not available in sandboxed builds.
:::
