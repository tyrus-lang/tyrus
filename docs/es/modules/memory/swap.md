---
title: swap
description: Intercambia dos bindings mutables del mismo tipo Copy.
---

# swap

<SignatureBlock label="Firma">swap&lt;T&gt;(left: T, right: T): void</SignatureBlock>

`swap` intercambia los valores de dos bindings locales mutables distintos del mismo tipo. Admite valores `Copy` y `string`; intercambiar strings transfiere sus asignaciones de propiedad entre los dos bindings sin copiar su contenido.

<!-- @include: ../../../.fragments/modules/memory/swap.md -->
