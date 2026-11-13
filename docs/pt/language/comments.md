---
title: Comentários
description: Comentários de linha, comentários de bloco e documentation comments em tyrus.
---

# Comentários

Tyrus suporta comentários de linha e de bloco. Comentários são descartados pelo lexer e não afetam análise semântica nem geração de código nativo.

## Comentários de linha

<!-- @include: ../../.fragments/language/comments/line-comments.md -->

## Comentários de bloco

<!-- @include: ../../.fragments/language/comments/block-comments.md -->

## Documentation comments

Documentation comments anexam texto estruturado a declarações. O Language Server usa esse conteúdo em hover e completion.

<!-- @include: ../../.fragments/language/comments/doc-comments.md -->

Tags suportadas incluem `@param`, `@returns`, `@deprecated` e `@example`.

::: tip Comentários dentro de strings são texto
Delimitadores de comentário dentro de strings com aspas e template literals são conteúdo de string, não comentários.
:::

<!-- @include: ../../.fragments/language/comments/string-comments.md -->