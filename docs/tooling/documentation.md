---
title: Documentation Maintenance
description: Release checklist and maintenance policy for the tyrus documentation site.
---

# Documentation Maintenance

The tyrus documentation is versioned with the compiler and runtime. Current documentation lives in `docs/`, archived releases are generated from Git tags, and the published site should expose the latest stable documentation at `/`.

## Ownership

The core maintainers own documentation version metadata, release checklist updates, and review of pages that describe language, compiler, runtime, CLI, formatter, linter, LSP, or native module behavior.

Translation ownership follows the same rule until dedicated translation maintainers exist. English remains the source language for technical behavior, with localized pages either translated completely or pointing to the English canonical page.

## Release Checklist

- Update documentation for every user-visible change before creating the release tag.
- Keep current docs in `docs/` and do not create permanent `docs/v0.1.0/` folders on the main branch.
- Confirm `docs/.vitepress/versions.ts` has the current version as `latestDocsVersion` and archived versions in `previousDocsVersions`.
- Run `npm run docs:build` before every release tag.
- Run `npm run docs:build:versions` before publishing the documentation site.
- Verify `/`, `/pt/`, `/es/`, version links, sidebar links, logo assets, favicon assets, and local search in the built site.
- Publish only `docs/.vitepress/dist` or the provider-specific static output directory.

## Review Rules

- Do not document incomplete or experimental behavior as stable.
- Mark experimental features explicitly and describe the currently implemented boundary.
- Keep API pages near the public module tree, such as `docs/modules/fs/read-file.md`, so sidebar links, search results, and source ownership remain predictable.
- Do not generate API reference pages automatically until the compiler or LSP exposes stable metadata for signatures, diagnostics, module exports, and examples.
- Prefer small pages with stable deep links over one large API page.
- Keep snippets executable or directly aligned with an executable example when the feature has runtime behavior.

## Future Generated Reference

Generated reference documentation is intentionally deferred until tyrus exposes stable compiler or LSP metadata. The future generator should read structured module exports, signatures, diagnostic codes, and examples instead of scraping Markdown or source comments.
