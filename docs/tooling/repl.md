---
title: REPL
description: Interactive console for exploring, prototyping, and learning tyrus.
---

# REPL

REPL stands for **Read-Eval-Print Loop**: a program that reads an input, evaluates it, prints the result, and repeats.

`tyrus repl` starts an interactive console where you write and evaluate tyrus code line by line. It is useful for exploring APIs, testing expressions, and learning the language without creating a project.

## Start the REPL

<!-- @include: ../.fragments/tooling/repl-start.md -->

The transcript above shows the current startup banner and the exact output of `:help`.

The REPL requires an interactive terminal. It refuses to run in a non-TTY environment such as a pipe.

## Features

### Syntax Highlighting

Code is highlighted in real time as you type, mirroring the classic Monokai color scheme: control keywords, operators and most other keywords in pink/red, storage/type keywords (`const`, `let`, primitive types, `interface`/`enum`/`type`, `=>`) in cyan, strings and template literals in yellow, numbers and booleans in purple, function calls in green, identifiers in off-white, and comments in gray.

### Auto-Completion

Press Tab to complete commands, keywords, primitive types, and symbols defined in the current session. Commands are completed after typing `:` (e.g. `:lo` → `:load `). File paths are completed for `:load` and `:restore`; `:restore` also completes the names of previously saved sessions. Inside member access, members are completed after `.` or `?.`: typing `console.` suggests `log`, `error`, and `warn`, and the name of an enum from the session or a native import (e.g. `HttpStatus.`) suggests its members.

### Multi-Line Input

The REPL detects incomplete input with the parser. When a line ends mid-construct (for example an unclosed `{`, `[`, or `(`), the prompt changes to `.... ` for continuation. A genuine parse error, such as a stray `;` inside an unclosed call, is reported immediately instead of waiting for more lines. Press Enter on an empty line to evaluate the accumulated input.

### Command History

Previous commands are saved to `~/.tyrus_repl_history` (up to 5000 entries) and are available with the Up/Down arrow keys. The history file persists across sessions.

## Commands

Every meta-command starts with `:`.

| Command | Purpose |
|---------|---------|
| `:help` | Show available commands |
| `:clear` | Clear the screen and session buffer |
| `:load <file>` | Load and evaluate a `.tyr` file |
| `:list` | List sessions saved via `:save` |
| `:type <expr>` | Show the inferred type of an expression |
| `:doc <symbol>` | Show documentation for a symbol |
| `:edit` | Edit session buffer in external editor (`$EDITOR`) |
| `:save <name.tyr>` | Save session buffer to `~/.tyrus/repl/<name>.tyr` |
| `:restore <name\|file> [--merge\|-m]` | Restore a saved session or a file, resetting the session first (validates code) |
| `:delete <name>` | Delete a saved session from `~/.tyrus/repl` |
| `:reset` | Reset the entire session |
| `:exit` | Exit the REPL |
| `:quit` | Alias for `:exit` |
| `:q` | Alias for `:exit` |

## Session Management

The REPL preserves your session state across commands. Variables, functions, and type aliases defined during the session remain available as you enter new code.

Sessions are `.tyr` files stored in `~/.tyrus/repl/`, one per saved session, so they survive across REPL invocations and are independent of the current working directory. `:save <name.tyr>` writes the session buffer to `~/.tyrus/repl/<name>.tyr` and requires the `.tyr` extension; `:restore <name>` reloads a saved session (the extension is optional, and an existing file path also works) and replays it the way it was typed: each statement is echoed as a `tyrus> ` transcript with syntax highlighting and then run, so every program output appears right below the line that produced it. By default `:restore` resets the session first, so the restored code always runs the way it did at `:save` time; pass `--merge` (or `-m`) to keep the current buffer and merge the restored code into it instead. With `--merge`, a name already defined on either side aborts the restore with an error before anything runs, leaving the session unchanged, resolve the conflict with `:reset` and try again. `:delete <name>` removes a saved session (the extension is optional too). `:list` shows the saved sessions with their line count, size, and when they were saved, newest first. If a restored unit contains invalid code, the restore is rejected and the session is left unchanged.

`:load <file>` replays a project file into the current session the same way `:restore` does: each statement is echoed as a `tyrus> ` transcript and then run, instead of only reporting a line count.

`:reset` clears the buffer and returns to a clean state. `:clear` clears the screen without discarding the buffer.

## Type Inspection

The `:type` command shows the inferred type of any expression without evaluating it:

<!-- @include: ../.fragments/tooling/repl/type-inspection.md -->

## Documentation Lookup

The `:doc` command displays documentation for symbols defined in the session, including parameter names, return types, `@param`, `@returns`, and `@deprecated` annotations, and usage examples:

<!-- @include: ../.fragments/tooling/repl/documentation-lookup.md -->

## External Editor

The `:edit` command opens the session buffer in the editor specified by `$EDITOR` or `$VISUAL` (defaults to `nano`). After the editor exits, the modified buffer is compiled, if there are errors, the session reverts to its previous state.

## Compilation

Each input is compiled from scratch using the in-process LLVM JIT compiler, which makes evaluation instant. When the JIT is unavailable (no LLVM runtime), the REPL falls back to compiling a native binary via clang.

The session buffer is wrapped in a `main` function (`const main = (): void => { ... };`) so function declarations and variable bindings coexist in the same scope.
