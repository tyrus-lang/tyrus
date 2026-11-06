---
title: VS Code
description: tyrus editing experience in VS Code with diagnostics, completion, hover, inlay hints, semantic tokens, Quick Fixes, and Quick Fix code lenses.
---

# VS Code

The VS Code extension is a client for the shared `tyrus lsp` server. It should feel like a native language experience while keeping semantic rules in the compiler and Language Server.

## Install and Open

Build or install the extension from `editors/vscode`, then open a project that contains `.tyr` files.

Install the published [Tyrus extension from the VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=tyrus-lang.tyrus).

<!-- @include: ../.fragments/tooling/install.md -->

The extension automatically selects itself as the default formatter for `.tyr` files and enables format on save. These defaults do not add entries to `settings.json`, and explicit user or workspace settings take precedence. Open Settings with `Ctrl+,` and search for `tyrus`, or search for `@ext:tyrus.tyr-language-support` to show only the extension settings.

The tyrus logo identifies the extension in the Extensions view, and `.tyr` files expose the tyrus language icon in the Explorer. A third-party file icon theme may intentionally replace language-contributed icons with its own artwork.

::: tip Shared intelligence
VS Code, Neovim, Zed, and IntelliJ/WebStorm should receive the same diagnostics and code intelligence from `tyrus lsp`.
:::

## Diagnostics

Compiler and linter diagnostics appear as squiggles in the editor and in the Problems panel. By default, the extension also adds a translucent full-line background: red for errors and yellow for warnings. The squiggle and its precise source range remain visible.

Set `tyrus.diagnostics.highlightWholeLine` to `false` in `settings.json` to disable the full-line background without changing diagnostics. Alternatively, press `Ctrl+,`, search for `tyrus`, find **Tyrus › Diagnostics: Highlight Whole Line**, and clear the checkbox.

::: danger error
<!-- @include: ../.fragments/tooling/diagnostics.md -->
:::

Expected editor feedback:

| Source | Severity | Example |
|--------|----------|---------|
| Compiler | Error | `expected i32, found string` |
| Linter | Warning | `retries is never reassigned` |
| Formatter/lint config schema | Error or warning | Unknown or invalid `tyrus-fmt.json` and `tyrus-lint.json` keys. |

## Completion

Completion includes local symbols, imported symbols, native module exports, keywords, primitive types, validator decorators, object fields, route option keys such as `handler`, `response`, `params`, and `config`, nested rate-limit keys such as `max`, `timeWindow`, and `globalMax`, and members for arrays, strings, `Regex`, `HttpRouter`, and `HttpStatus`. Hovering a route option explains its expected value and purpose.

<!-- @include: ../.fragments/tooling/completion.md -->

## Hover and Signature Help

Hover shows inferred types, native import signatures, interface fields, validator documentation, and diagnostics. Signature help shows parameter names and the active argument while filling a call.

<!-- @include: ../.fragments/tooling/hover.md -->

::: details Example hover
<!-- @include: ../.fragments/tooling/hover-example.md -->
:::

## Inlay Hints

The Language Server provides parameter-name inlay hints and inferred-type hints.

<!-- @include: ../.fragments/tooling/inlay-hints-source.md -->

Expected hints:

<!-- @include: ../.fragments/tooling/inlay-hints-expected.md -->

## Quick Fixes

Quick Fixes are available when the edit is deterministic.

<!-- @include: ../.fragments/tooling/quick-fix.md -->

Examples include importing a missing symbol, removing unused imports, applying `prefer-const`, and borrowing a moved value instead of moving it.

## Quick Fix Code Lenses

When a `.tyr` diagnostic offers at least one deterministic Quick Fix, the Language Server also surfaces it as a code lens above the line with the problem. The lens shows the exact Quick Fix title (for example, `Change let to const`), and clicking it applies the edit immediately without opening the Quick Fix menu. When one diagnostic has more than one fix, each fix gets its own lens beside the others. Quick Fixes for `.env` files remain available through the lightbulb only.

The regular Quick Fix keeps working through the lightbulb and the editor's standard commands; lenses only add an entry point closer to the code.

Set `tyrus.codeLens.quickFixes.enable` to `false` in `settings.json` to hide Quick Fix code lenses without changing the Quick Fixes themselves. Changing this setting takes effect after reloading the window.

## Semantic Highlighting

Semantic tokens improve coloring for symbols, primitive types, native types, interfaces, type aliases, keywords, literal keywords, and words reserved for future language features.

::: warning Highlighting is not semantic validation
Syntax colors help readability, but compiler and linter diagnostics are the source of truth for correctness.
:::
