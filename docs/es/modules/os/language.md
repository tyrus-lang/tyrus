---
title: language
description: Lee el subtag de idioma del locale actual mediante el módulo nativo tyrus:os.
---

# language

`language` devuelve el subtag de idioma del locale actual (ej.: `"en"`, `"pt"`), la parte inicial del código completo de [`locale`](./locale.md).

<SignatureBlock label="Firma">language(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-language.md -->
::: tip Nunca falla el programa
Recurre a `"und"`, el subtag estándar de "locale indeterminado", si el locale reportado por el SO no puede leerse, en lugar de abortar el programa.
:::
