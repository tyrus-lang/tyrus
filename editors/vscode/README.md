# Tyrus Language Support

<p align="center"><img src="https://tyruslang.org/tyrus-logo.png" alt="tyrus logo" width="160"></p>

Language Server integration, syntax highlighting, diagnostics, navigation, and IntelliSense for **tyrus**.

## Features

The extension currently provides:

- Language Server startup through the `tyrus lsp` command;
- the **Tyrus: Restart Language Server** command to restart diagnostics, hover, and completion after rebuilding the compiler;
- parser, semantic, ownership, import, and type diagnostics from the native tyrus analysis pipeline;
- translucent full-line highlighting for errors and warnings while preserving the standard diagnostic underline;
- completion for local symbols, imported symbols, native exports, and function call snippets;
- member completion for arrays, strings, `Regex`, `HttpRouter`, `HttpStatus`, and `HttpError`;
- hover information for variables, parameters, functions, imports, and inferred expression types;
- signature help showing parameter names while filling in a call;
- go to definition within the current file and across imported local modules;
- find all references to a symbol across the whole project;
- **Rename Symbol** (`F2`) across the project, updates the definition, every `import` (the imported name, keeping any `as` alias), and every use in one refactor. Typing over a name by hand only edits that spot; use `F2` (or right-click → "Rename Symbol") to propagate the rename to the imports;
- automatic imports for project exports and native exports;
- Quick Fix code actions for identifiers that can be imported;
- parameter name inlay hints at call sites;
- semantic highlighting for words reserved for future language features;
- document formatting through the language server (`tyrus-fmt`), gated by `tyrus.format.enable`;
- hover, completion, signature help, and diagnostics for the declarative `tyrus:http` API, including `HttpReply<T>`, `reply.status(HttpStatus.X).send(...)`, `reply.error(HttpError.X(...))`, and `HttpError.custom(status, errorCode, message)`;
- keyword, literal, primitive type, native type, interface, and type alias completions;
- syntax highlighting and completions for the full numeric type set: `i32`, `i64`, `i128`, `isize`, `u32`, `u64`, `u128`, `usize`, `f32`, `f64`, and `f128`;
- syntax highlighting and language server support for `as const` const assertions and `readonly` type annotations;
- syntax highlighting and LSP diagnostics for optional object properties, excess object properties, and immutable object updates;
- language registration for `.tyr` files;
- syntax highlighting through the bundled TextMate grammar;
- editor configuration for comments, brackets, auto-closing pairs, surrounding pairs, and indentation;
- tyrus branding for the installed extension and a language icon for `.tyr` files when the active file icon theme honors language-contributed icons;
- JSON Schema validation, hover, and completion for `tyrus-fmt.json` and `tyrus-lint.json`.

## Navigation and Quick Fixes

Place the cursor on a local or imported symbol and press `F12`, or right-click and choose **Go to Definition**. On a diagnostic, press `Ctrl+.` on Windows/Linux or `Cmd+.` on macOS to open deterministic Quick Fixes. When enabled, Quick Fix code lenses above diagnostics apply the same actions with one click.

## Configuration

For `.tyr` files, the extension automatically provides these defaults without writing them to the user or workspace `settings.json`:

```json
"[tyrus]": {
  "editor.defaultFormatter": "tyrus-lang.tyrus",
  "editor.formatOnSave": true
}
```

User and workspace settings still take precedence. Open VS Code Settings with `Ctrl+,` and search for `tyrus`, or use `@ext:tyrus-lang.tyrus` to show only this extension's settings.

- `tyrus.server.path`: path to the `tyrus` executable used to start `tyrus lsp`. Defaults to `tyrus`.
- `tyrus.format.enable`: enables the `.tyr` document formatter. Defaults to `true`.
- `tyrus.diagnostics.highlightWholeLine`: highlights the full line behind errors and warnings while preserving the standard diagnostic underline. Defaults to `true`; set it to `false` to use only the editor's standard diagnostic presentation.
- `tyrus.codeLens.quickFixes.enable`: enables Quick Fix code lenses above lines with diagnostics. Defaults to `true`; set it to `false` to hide them. Takes effect after reloading the window.
- `tyrus.workspace.maxFiles`: maximum number of Tyr files indexed by workspace-wide features. Defaults to `10000`; valid values are `1` through `100000`.
- `tyrus.workspace.maxDepth`: maximum directory depth searched for Tyr files. Defaults to `64`; valid values are `1` through `256`.
- `tyrus.workspace.excludeDirectories`: additional directory names skipped during file discovery. Built-in exclusions such as `.git`, `target`, and `node_modules` remain active. Workspace scan settings take effect after restarting the language server.

### Disable Full-Line Diagnostic Highlighting

To keep only the standard diagnostic underline, add this setting to the user or workspace `settings.json`:

```json
{
  "tyrus.diagnostics.highlightWholeLine": false
}
```

To change it through the Settings UI, press `Ctrl+,`, search for `tyrus`, find **Tyrus › Diagnostics: Highlight Whole Line**, and clear the checkbox. The change applies immediately.
