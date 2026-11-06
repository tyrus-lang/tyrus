---
title: Path Aliases
description: Static directory aliases for internal imports.
---

# Path Aliases

Declare static aliases for internal source directories in the `imports` object of `tyrus.json`.

<!-- @include: ../.fragments/docs/path-aliases.md -->

Every alias key begins with `#` and maps a directory relative to the project root. Use `#` for a conventional `src/` directory, then import its children with `#/folder`.

Use `#config` to import `src/config/index.tyr`, or `#config/mongodb` to import a child module. An alias never targets a source file directly, expands another alias, leaves the project root, or resolves at runtime.

When more than one alias applies, the longest prefix wins. An unresolved `#` specifier is a compilation error and editor diagnostic.
