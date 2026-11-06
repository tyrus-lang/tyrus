---
title: Comentarios
description: Comentarios de línea, comentarios de bloque y documentation comments en tyrus.
---

# Comentarios

Tyrus soporta comentarios de línea y de bloque. Los comentarios son descartados por el lexer y no afectan el análisis semántico ni la generación de código nativo.

## Comentarios de línea

<!-- @include: ../../.fragments/language/comments/line-comments.md -->

## Comentarios de bloque

<!-- @include: ../../.fragments/language/comments/block-comments.md -->

## Documentation comments

Documentation comments anexan texto estructurado a declaraciones. El Language Server usa ese contenido en hover y completion.

<!-- @include: ../../.fragments/language/comments/doc-comments.md -->

Tags soportadas incluyen `@param`, `@returns`, `@deprecated` y `@example`.

::: tip Comentarios dentro de strings son texto
Los delimitadores de comentario dentro de strings con comillas y template literals son contenido de string, no comentarios.
:::

<!-- @include: ../../.fragments/language/comments/string-comments.md -->
