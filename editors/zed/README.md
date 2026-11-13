# Tyrus for Zed

Zed extension that connects the editor to the `tyrus lsp` language server, so diagnostics, completion, hover, go to definition, find references, rename, signature help, and semantic-token highlighting all come from the tyrus compiler.

## Status

This is a **scaffold**: it wires the language server and references the Tree-sitter grammar, but is not built or tested in this repository's CI, because a Zed extension is a separate WebAssembly crate (`zed_extension_api`, target `wasm32`) built by Zed's dev-extension flow.

- `extension.toml`, `languages/tyrus/config.toml`, `Cargo.toml`, and `src/lib.rs` follow Zed's documented extension pattern to register the `tyrus lsp` language server.
- Syntax highlighting uses `tree-sitter-tyrus` (in `editors/tree-sitter-tyrus`, validated against the whole example corpus), plus the LSP semantic tokens for parameters and reserved-for-future keywords.
- **Before publishing:** set `commit` under `[grammars.tyr]` in `extension.toml` to the repository SHA that contains `editors/tree-sitter-tyrus`.

## Install as a dev extension

1. Build the `tyrus` compiler (`./scripts/tyrus`) and make sure `tyrus` is on your `PATH`.
2. In Zed: `zed: install dev extension` and select this `editors/zed` directory.

## Go to Definition and Quick Fix

Place the cursor on a local or imported symbol and run **Go to Definition** from Zed's command palette. To repair a diagnostic, open its code actions from the editor and choose a deterministic Quick Fix. Both requests are handled by `tyrus lsp` and use the same declarations and diagnostic ranges as the other editor clients.

## How to test

1. From the repo root run `./scripts/tyrus` (compiler + `tyrus` on `PATH`) and `./scripts/editors` (regenerates the grammar and highlights).
2. Set `commit` under `[grammars.tyr]` in `extension.toml` to the current repo SHA (`git rev-parse HEAD`).
3. In Zed run `zed: install dev extension` and pick `editors/zed` (Zed builds the WASM extension and the tree-sitter grammar).
4. Open `src/main.tyr` in a project created with `tyrus new`. Check tree-sitter highlighting for keywords, types, strings, and functions, and verify that the LSP provides hover, completion, diagnostics, rename, and references.
5. After editing the grammar or extension, run `zed: reload extensions`.
