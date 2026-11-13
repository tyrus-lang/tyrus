---
title: locale
description: Lee el código de locale actual mediante el módulo nativo tyrus:os.
---

# locale

`locale` devuelve el código de locale actual (ej.: `"en-US"`, `"pt-BR"`).

<SignatureBlock label="Firma">locale(): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-locale.md -->
::: tip Nunca falla el programa
Recurre a `"und"`, el subtag estándar de "locale indeterminado", si el locale reportado por el SO no puede leerse, en lugar de abortar el programa.
:::

Consulta también [`language`](./language.md), que devuelve solo el subtag de idioma inicial.
