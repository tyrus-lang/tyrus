---
title: replace
description: Almacena un valor Copy y devuelve el valor anterior.
---

# replace

<SignatureBlock label="Firma">replace&lt;T&gt;(destination: T, source: T): T</SignatureBlock>

`replace` almacena `source` en un binding local mutable y devuelve el valor que estaba almacenado antes. Admite tipos `Copy` y `string`. Para `string`, los dos argumentos deben ser bindings locales mutables; `source` se mueve y no puede usarse después de la llamada.

<!-- @include: ../../../.fragments/modules/memory/replace.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/memory/replace-error.md -->
:::
