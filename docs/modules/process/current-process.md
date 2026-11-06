---
title: Current Process
description: Read arguments and process information or set the program exit code.
---

# Current Process

The ambient `process` namespace describes the running Tyrus program. These values are available without importing `tyrus:process`; that module is used to start and manage child processes.

## API Reference

| Member | Type or signature | Description |
| --- | --- | --- |
| `process.argv` | `string[]` | Command-line arguments. Index `0` is the executable path; user arguments begin at index `1`. |
| `process.pid` | `u32` | Process identifier of the running program. |
| `process.ppid` | `u32` | Process identifier of its parent. |
| `process.cwd()` | `() => string` | Returns the current working directory. |
| `process.uptime()` | `() => u64` | Returns elapsed runtime in milliseconds. |
| `process.memoryUsage()` | `() => MemoryInfo` | Returns resident memory usage in bytes. |
| `process.cpuUsage()` | `() => CpuUsage` | Returns cumulative user and system CPU time in microseconds. |
| `process.exit(code)` | `(code: i32) => never` | Ends the program with the given exit code. |

## Command-line Arguments

Arguments after `--` in `tyrus run` are passed to the program instead of the compiler. `Array.at()` returns `string?`, so handle an argument that may be absent explicitly.

<!-- @include: ../../.fragments/docs/environment/argv.md -->

Because a compiled Tyrus program is the executable, the user's first argument is at index `1`. Code ported from a runtime where index `0` is an interpreter and index `1` is the script should use `slice(1)`, not `slice(2)`.

## Process Information

`process.memoryUsage()` returns this value:

```tyrus
interface MemoryInfo {
    readonly rss: u64;
}
```

`rss` is the process's resident set size in bytes.

`process.cpuUsage()` returns this value:

```tyrus
interface CpuUsage {
    readonly user: u64;
    readonly system: u64;
}
```

`user` and `system` are cumulative CPU time in microseconds since the process started, not percentages. Unix targets use `getrusage`; Windows converts the `GetProcessTimes` 100-nanosecond counters to microseconds. Other targets return `0` for both fields.

<!-- @include: ../../.fragments/docs/environment/process-info.md -->

`process.exit(code)` ends the program immediately, so statements after it are unreachable. A program that returns normally exits with code `0`.
