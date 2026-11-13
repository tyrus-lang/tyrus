---
title: ProcessError
description: Inspect recoverable failures from process operations.
---

# ProcessError

`ProcessError` is the structured error type for recoverable process failures.

<SignatureBlock label="Signature">interface ProcessError { readonly code: string; readonly message: string }</SignatureBlock>

`ProcessError` is an `Error` value and can be thrown by user-defined functions. The native `exec` API returns it as the `Err` variant of `Result<ChildProcess, ProcessError>`.

## Fields

| Field | Type | Purpose |
| --- | --- | --- |
| `code` | `string` | Stable `ERR_PROCESS_*` identifier. |
| `message` | `string` | Human-readable explanation. |

## Usage and errors

<!-- @include: ../../.fragments/modules/process/errors.md -->

Start-time failures include invalid arguments, unavailable pipes, a file the program is not permitted to execute (`ERR_PROCESS_PERMISSION_DENIED`), and an executable that cannot start for any other reason, such as a command that does not exist (`ERR_PROCESS_SPAWN_FAILED`). A non-zero exit code and bytes on `stderr` are normal process results, not `ProcessError` values.
