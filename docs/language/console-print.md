---
title: Console / Print
description: Writing to stdout and stderr with console.log, console.error, console.warn, print, printError, printWarn, printRaw, and printCR.
---

# Console / Print

Tyrus offers two equivalent ways to write to the terminal: the `console` object and the ambient `print`/`printError`/`printWarn`/`printRaw`/`printCR` functions, available without any import.

## console.log

`console.log` writes to stdout. Every argument must be printable: primitives (`string`, the numeric types, `boolean`, `char`), regular expressions, arrays and tuples of printable values, and interfaces/objects, which print as `{ field: value, ... }`.

Multiple arguments are space-joined on a single line, with a trailing `\n` always appended.

<!-- @include: ../.fragments/language/console-print/console-log.md -->

## console.error

`console.error` writes to stderr. It accepts the same argument types as `console.log`, space-joins them the same way, and appends the same trailing `\n`.

<!-- @include: ../.fragments/language/console-print/console-error.md -->

## console.warn

`console.warn` writes to stderr. It accepts the same argument types as `console.log`, space-joins them the same way, and appends the same trailing `\n`.

<!-- @include: ../.fragments/language/console-print/console-warn.md -->

## Color in `warn` and `error`

In a terminal, `console.warn` colors its whole line yellow and `console.error` colors it red, opened before the first argument and reset after the last, so inter-argument spaces stay colored too:

<!-- @include: ../.fragments/language/console-print/color.md -->

::: tip Same precedence as styleText
Whether to colorize is decided once per process from `stderr`'s own terminal detection. `NO_COLOR` (non-empty) wins, then `FORCE_COLOR`, else TTY detection, the same precedence [styleText](/modules/utils/style-text) already documents. `console.log`, `print`, and `printRaw` never colorize.
:::

## print

`print` is an ambient global function, no import required, the same way `assert` is always available. It behaves exactly like `console.log`: the same accepted argument types, the same space-joined arguments, the same trailing `\n`, and the same destination (stdout).

<!-- @include: ../.fragments/language/console-print/print.md -->

::: tip print and console.log share the same code path
`print` and `console.log` compile down to the exact same sequence of writes. Neither is faster than the other, pick whichever reads better at the call site.
:::

## printError

`printError` is an ambient global function that behaves exactly like `console.error`: the same accepted argument types, the same space-joined arguments, the same trailing `\n`, and the same destination (stderr). In a terminal, it colors its whole line red with the same color precedence `console.error` uses.

<!-- @include: ../.fragments/language/console-print/print-error.md -->

::: tip printError shares the console.error code path
`printError` compiles down to the exact same emission as `console.error`. Reach for whichever reads better at the call site.
:::

## printWarn

`printWarn` is an ambient global function that behaves exactly like `console.warn`: the same accepted argument types, the same space-joined arguments, the same trailing `\n`, and the same destination (stderr). In a terminal, it colors its whole line yellow with the same color precedence `console.warn` uses.

<!-- @include: ../.fragments/language/console-print/print-warn.md -->

::: tip printWarn shares the console.warn code path
`printWarn` compiles down to the exact same emission as `console.warn`. Reach for whichever reads better at the call site.
:::

## printRaw

`printRaw` is `print` without the trailing `\n`. Consecutive calls land on the same terminal line, which is useful for progress indicators or building output incrementally:

<!-- @include: ../.fragments/language/console-print/print-raw.md -->

::: tip printRaw flushes explicitly
`print`/`console.log` never need to: they always end in `\n`, and the OS's own line buffering flushes on that by itself. `printRaw` doesn't end in `\n`, so tyrus flushes stdout explicitly right after it, without that, the text could sit in the buffer, invisible, until something else happened to flush it later.
:::

## printCR

`printCR` is `printRaw` that also erases whatever the *previous* `printCR` call left on the current line first, useful for progress bars, spinners, or any output that updates in place rather than scrolling:

<!-- @include: ../.fragments/language/console-print/print-cr.md -->

::: tip Stateless, and safe outside a terminal
Erasing works by writing `\r` (return to column 0) followed by the ANSI "clear to end of line" escape (`\x1B[K`), then the new text, no runtime bookkeeping of how long the previous line was. That escape sequence is skipped entirely when stdout is not a real terminal (a pipe, a CI log): it would not clear anything there, only corrupt the log with raw escape bytes, so `printCR` degrades to writing its text plainly, like `printRaw`, in that case.
:::

## Reserved names

`console`, `print`, `printError`, `printWarn`, `printRaw`, `printCR`, `assert`, `assertEqual`, and `assertThrows` are reserved. They lex as ordinary identifiers, not language keywords, but a declaration reusing one of these names, a function, variable, type, parameter, catch binding, import alias, or destructured name, is rejected (`E0003`), exactly like a real reserved keyword such as `const` or `throw`. This guarantees the same call always means the same thing, in every file:

::: danger error
<!-- @include: ../.fragments/language/console-print/reserved-names.md -->
:::
