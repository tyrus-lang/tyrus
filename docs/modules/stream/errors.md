---
title: StreamError
description: Inspect recoverable failures from stream operations.
---

# StreamError

`StreamError` is the structured error type used by recoverable stream operations.

<SignatureBlock label="Signature">interface StreamError { readonly code: string; readonly message: string }</SignatureBlock>

`StreamError` is an `Error`: a `catch (error)` without an annotation receives it, and `error is StreamError` narrows the binding to it, with every field of `StreamError`.

## Fields

| Field | Type | Purpose |
| --- | --- | --- |
| `code` | `string` | Stable identifier for programmatic handling. |
| `message` | `string` | Human-readable explanation. |

## Usage and errors

<!-- @include: ../../.fragments/modules/stream/errors.md -->

Use `throws StreamError` on an enclosing function when it forwards an operation that can reject with this error.
