---
title: exec
description: Start an external program without invoking a shell.
---

# exec

`exec` starts one external program and connects its standard descriptors to bounded byte streams.

<SignatureBlock label="Signature">exec(command: string, args?: string[]): Result&lt;ChildProcess, ProcessError&gt;</SignatureBlock>

## Parameters

| Parameter | Type | Purpose |
| --- | --- | --- |
| `command` | `string` | Executable path or command name. |
| `args` | `string[]?` | Individual arguments, or an omitted empty list. |

## Usage and errors

<!-- @include: ../../.fragments/modules/process/exec.md -->

Each argument is passed directly to the program. `exec` does not parse shell quoting, redirections, or pipelines. A start-time failure returns `Err(ProcessError)`.
