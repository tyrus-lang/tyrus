---
title: EOL
description: The native line-ending constant from the native tyrus:os module.
---

# EOL

`EOL` is the operating system's native line-ending sequence: `"\r\n"` on Windows, `"\n"` everywhere else.

<SignatureBlock label="Signature">EOL: string</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-eol.md -->
::: tip A constant, not a function call
Unlike every other export of `tyrus:os`, `EOL` is a plain value, not a function, write `EOL`, not `EOL()`. It resolves to a fixed value when the compiler itself was built, so referencing it never involves a runtime lookup.
:::
