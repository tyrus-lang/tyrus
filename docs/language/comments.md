---
title: Comments
description: Line comments, block comments, and documentation comments in tyrus.
---

# Comments

Tyrus supports line and block comments. Comments are discarded by the lexer and do not affect semantic analysis or native code generation.

## Line Comments

<!-- @include: ../.fragments/language/comments/line-comments.md -->

## Block Comments

<!-- @include: ../.fragments/language/comments/block-comments.md -->

## Documentation Comments

Documentation comments attach structured text to declarations. The Language Server uses them in hover and completion.

<!-- @include: ../.fragments/language/comments/doc-comments.md -->

Supported tags include `@param`, `@returns`, `@deprecated`, and `@example`.

::: tip Comments inside strings are text
Comment delimiters inside quoted strings and template literals are string content, not comments.
:::

<!-- @include: ../.fragments/language/comments/string-comments.md -->
