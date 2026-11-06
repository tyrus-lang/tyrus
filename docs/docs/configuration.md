---
title: Project Configuration
description: Project structure, tyrus.json, documentation scripts, and release workflow.
---

# Project Configuration

A generated tyrus project has a small manifest, an entry point, optional typed environment files, and a `target/` directory for generated artifacts.

## Project Structure

<!-- @include: ../.fragments/docs/configuration-project-structure.md -->

This is the default `script` project. The `cli` template adds `src/prompts/setup.tyr` and uses `__tests__/example.test.tyr` instead. The `api` template adds `src/controllers/user.tyr`, `src/models/user.tyr`, `src/repositories/user.tyr`, `src/routes/user.tyr`, `src/schemas/user.tyr`, and `src/services/user.tyr`, and also uses `__tests__/example.test.tyr`. The `api` template creates a `data/` directory when SQLite is selected.

`src/main.tyr` is the required entry file and must declare `const main = (): void => { ... };`. The path and function signature are fixed. Build artifacts are written to `target/debug/` by default and to `target/release/` with `tyrus build --release`. `tyrus dev` uses `target/debug/`.

## tyrus.json

<!-- @include: ../.fragments/docs/configuration-tyrus-json.md -->

`project` describes package metadata. `build.optimize` selects the default optimization mode. `development.https` makes a server started without `tls` serve HTTPS on `localhost` while the program runs with `TYRUS_ENV=development`, and does nothing in any other environment; see [createRouter](../modules/http/create-router.md). `runtime.workerThreads` limits how many worker threads the asynchronous runtime may use, and defaults to the number of logical CPUs when omitted. `security.envFile` enables typed environment analysis. `security.envSealed` switches runtime environment loading to an encrypted sealed artifact.

`scripts` maps project script names to shell commands. Generated manifests include `fmt`, `lint`, and `test` entries. Add a custom script as a string value, then run it by name with `tyrus run <name>`. For example, this entry makes `tyrus run example` execute `tyrus run example.tyr` from the project directory:

```json
{
  "scripts": {
    "example": "tyrus run example.tyr"
  }
}
```

Arguments for the configured command follow `--`, as in `tyrus run example -- --verbose`. `tyrus --init` preserves custom scripts while adding missing defaults; `tyrus --default` resets the manifest to generated project defaults and removes custom scripts.

## Documentation Site

The repository documentation lives in `docs/` and is built with VitePress:

<!-- @include: ../.fragments/docs/configuration-docs-commands.md -->

Current docs are published at `/`. Archived releases are generated from Git tags into paths such as `/v0.1.0/`. Version metadata lives in `docs/.vitepress/versions.ts`.
