---
title: Playground
description: Write, compile, and run tyrus code directly in the browser.
---

# Playground

The Tyrus Playground is a web-based editor for writing, compiling, and executing Tyrus code without installing anything locally. It compiles your code to WebAssembly on the server and runs it in a sandboxed Web Worker in the browser.

## Access

Open [playground.tyruslang.org](https://playground.tyruslang.org) in any modern browser.

## Features

- **Monaco Editor** with custom Tyrus syntax highlighting.
- **Dark and light themes** via the toggle button.
- **Pre-loaded examples** (Hello World, Fibonacci, Match, Loops, Const, Enums).
- **Ctrl+Enter** to compile and run.
- **Output panel** showing stdout, stderr, and compilation errors.
- **Share links** via short URLs (`/s/{shortId}`).

## How It Works

1. You write code in the editor.
2. Clicking **Run** sends the code to the backend.
3. The backend compiles it to WebAssembly using `tyrus build --compile-target wasm`.
4. The WASM binary is loaded into a sandboxed Web Worker.
5. The Worker executes the code and captures stdout/stderr.
6. Output is displayed in the output panel.

## What Runs in the Sandbox

The playground runs your program in the browser, so anything that needs an operating system behind it cannot work there. Everything else does, the language itself, the standard library, and the parts of the runtime that are pure computation.

Available: strings and regular expressions, arrays and their functional methods, `Map` and `Set`, `decimal`, JSON, math, validators, and `tyrus:temporal` including timezone conversion.

::: warning `tyrus:cli`, `tyrus:crypto`, `tyrus:fs`, `tyrus:http`, `tyrus:net`, `tyrus:os`, `tyrus:process`, `tyrus:stream`, and `tyrus:task` do not run in the playground
Every one of these needs real operating-system access the browser sandbox does not have, a thread pool for `tyrus:task`'s parallelism, the filesystem for `tyrus:fs`, sockets for `tyrus:net`/`tyrus:http`, and so on. Importing any of them is rejected when the program is compiled, naming the module (`module `tyrus:task` is not available in a sandboxed build`, for example). The ambient `process` namespace has no import to reject, so it reports itself unavailable when a program reaches for it. The same refusal applies to any build targeting WebAssembly, not only the playground's, because the target is what lacks the access rather than the sandbox; without it the program reached the linker and failed there with an undefined runtime symbol instead of a message naming the import. Run examples using any of these locally with `tyrus run` instead.
:::

Reaching an ambient API that the sandbox cannot provide does not crash the program silently. Depending on the API, it either raises a catchable error whose `code` follows the `ERR_<MODULE>_UNAVAILABLE` form, or stops the program with a message naming what it reached for. Either way the reason appears in the output panel.

<!-- @include: ../.fragments/tooling/playground/what-runs-in-the-sandbox.md -->

<!-- @include: ../.fragments/tooling/playground/what-runs-in-the-sandbox-2.md -->

::: warning `async`/`await` does not run in the playground
Asynchronous code compiles, but there is nothing to drive it: the sandbox has no task executor, and `main()` runs to completion synchronously. A program that awaits will not produce the result you expect. Run it locally with `tyrus run` instead.

This is a property of the sandbox, not a gap in the language, asynchronous code works normally outside the playground.
:::

## Limitations

- Code is limited to 8 KB.
- Execution is limited to 5 seconds.
- WASM memory is capped at 8 MB.
- File system, network, OS modules and `async`/`await` execution are unavailable in the sandbox.
- Compilation is rate-limited to 5 requests per minute per IP.
