---
title: Errors and Diagnostics
description: Compiler, linter, runtime, and editor diagnostics with examples.
---

# Errors and Diagnostics

tyrus diagnostics use stable codes, precise source ranges, related labels, and actionable help when a concrete fix exists. The same compiler analysis feeds the CLI and the Language Server.

::: tip Read the range first
Diagnostics should point to the token, expression, declaration, or initializer that caused the problem. When the editor shows a squiggle, start at that highlighted range.
:::

## Compiler Errors

Type errors stop compilation before LLVM code generation.

::: danger error
<!-- @include: ../.fragments/language/errors/compiler-error-code.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/errors/compiler-error-output.md -->
:::

## Borrow and Move Errors

Non-Copy values move by default. After a move, the previous binding is invalid.

::: danger error
<!-- @include: ../.fragments/language/errors/borrow-move-code.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/errors/borrow-move-output.md -->
:::

::: warning Ownership is enforced before runtime
Move and borrow errors are compile-time failures. The generated binary never needs a garbage collector to recover from these mistakes.
:::

## Linter Warnings

Lint warnings report code that is valid but risky, noisy, or inconsistent with project architecture.

::: danger error
<!-- @include: ../.fragments/language/errors/linter-warning-code.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/errors/linter-warning-output.md -->
:::

When the fix is deterministic, the Language Server exposes the same edit as a Quick Fix.

## Runtime Failures

Generated programs write runtime failures to stderr and exit with a non-zero status. Normal `print` output stays on stdout.

::: danger error
<!-- @include: ../.fragments/language/errors/runtime-failure-code.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/errors/runtime-failure-output.md -->
:::

Runtime failures include safety checks inserted by the compiler, such as overflow, division by zero, invalid narrowing, forced optional unwrap, and validation failures.

## Validation Failures

Validator decorations can fail at compile time for literals and at runtime for dynamic values.

::: danger error
<!-- @include: ../.fragments/language/errors/validation-failure-code.md -->
:::

::: danger error
<!-- @include: ../.fragments/language/errors/validation-failure-output.md -->
:::

::: danger Validation aborts use stderr
Runtime validation failures call the runtime validation panic helper, print the validation diagnostic to stderr, and stop the process with a non-zero exit status.
:::

## Editor Diagnostics

VS Code, Neovim, Zed, and IntelliJ/WebStorm integrations should all receive diagnostics from `tyrus lsp`. Editor-specific clients should not reimplement type rules.

## Error Codes

The table lists every public `E####` compiler diagnostic. Use `tyrus explain <code>` for a concise terminal reference; diagnostic messages, labels, and available Quick Fixes remain context-dependent.

<!-- @include: ../.fragments/language/errors/error-codes.md -->
