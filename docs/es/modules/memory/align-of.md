---
title: alignOf
description: Alineación en bytes, en tiempo de compilación, de un tipo primitivo.
---

# alignOf

<SignatureBlock label="Firma">alignOf&lt;T&gt;(): usize</SignatureBlock>

`alignOf` devuelve el límite en bytes requerido para almacenar un tipo primitivo. Un `f64` tiene alineación `8`, por lo que su dirección debe ser múltiplo de 8. El compilador inserta padding dentro de structs cuando es necesario para cumplir este requisito. Se evalúa en tiempo de compilación.

<!-- @include: ../../../.fragments/modules/memory/align-of/alignof.md -->

Acepta tipos enteros, tipos de punto flotante, `boolean` y `char`. Los valores gestionados, como `string` y arrays dinámicos, se rechazan.
