---
title: styleText
description: ANSI text styling through the native tyrus:utils module.
---

# styleText

`styleText` wraps text in ANSI escape sequences for terminal output.

<SignatureBlock label="Signature">styleText(format, text): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/utils/style-text/styletext.md -->
The compiler recognizes multiple call shapes. Style names must be string literals or static style members so invalid styles are reported at compile time.

::: warning Use stderr for terminal chrome
Styled status messages should usually go to stderr. Keep stdout for normal data output.
:::
