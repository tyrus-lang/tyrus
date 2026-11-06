# Tyrus for Neovim

Neovim support for **tyrus** reuses the same `tyrus lsp` language server as the VS Code extension, so diagnostics, hover, completion, go to definition, find references, rename, signature help, inlay hints, formatting, and semantic-token highlighting all come from the compiler, nothing is reimplemented here.

This directory provides only the editor glue:

- `ftdetect/tyrus.vim`, associates `*.tyr` with the `tyrus` filetype;
- `syntax/tyrus.vim`, base keyword/type/string highlighting. It is **generated** from the compiler lexicon by `cargo run -p xtask`; do not edit it by hand.

## Requirements

- Neovim 0.9+ (LSP semantic tokens are applied automatically from 0.9);
- the `tyrus` executable on your `PATH` (build it with `./scripts/tyrus`).

## Install

Point your plugin manager at this directory, or add it to the runtime path directly. With [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{ dir = "/path/to/tyrus/editors/neovim" }
```

Or without a plugin manager, in `init.lua`:

```lua
vim.opt.runtimepath:append("/path/to/tyrus/editors/neovim")
```

## Start the language server

On Neovim 0.11+ (built-in `vim.lsp.config`):

```lua
vim.lsp.config.tyr = {
  cmd = { "tyrus", "lsp" },
  filetypes = { "tyrus" },
  root_markers = { "tyrus.json", ".git" },
}
vim.lsp.enable("tyrus")
```

On older Neovim, start it per buffer:

```lua
vim.api.nvim_create_autocmd("FileType", {
  pattern = "tyrus",
  callback = function(args)
    vim.lsp.start({
      name = "tyrus",
      cmd = { "tyrus", "lsp" },
      root_dir = vim.fs.root(args.buf, { "tyrus.json", ".git" }),
    })
  end,
})
```

With the server attached you get the full feature set over LSP, including project-wide **rename** (`vim.lsp.buf.rename()`) and **references** (`vim.lsp.buf.references()`). Formatting uses `vim.lsp.buf.format()`.

## Go to Definition and Quick Fix

Neovim does not impose LSP keybindings. Add these buffer-local mappings to your `LspAttach` callback to navigate to a definition with `gd` and open Quick Fixes with `<leader>ca`:

```lua
vim.api.nvim_create_autocmd("LspAttach", {
  callback = function(args)
    local opts = { buffer = args.buf }
    vim.keymap.set("n", "gd", vim.lsp.buf.definition, opts)
    vim.keymap.set("n", "gr", vim.lsp.buf.references, opts)
    vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action, opts)
  end,
})
```

Use `gd` on an imported name to jump to its local module declaration. Use `<leader>ca` on a diagnostic to apply a deterministic compiler or linter fix.

## How to test

1. Build the compiler and language server, and put `tyrus` on your `PATH`: run `./scripts/tyrus` from the repo root.
2. Regenerate the editor artifacts (only needed after changing the lexicon/grammar): `./scripts/editors`.
3. Add `editors/neovim` to the runtime path and register the server (snippets above), then open `src/main.tyr` from a project created with `tyrus new`.
4. Check: keywords/types/strings are colored (from `syntax/tyrus.vim`); `K` shows hover types; `<C-x><C-o>` or your completion shows symbols; errors underline; `:lua vim.lsp.buf.rename()` renames across files; `:lua vim.lsp.buf.references()` lists uses.
