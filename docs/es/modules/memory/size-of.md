---
title: sizeOf
description: Tamaño en bytes, en tiempo de compilación, de un tipo primitivo.
---

# sizeOf

<SignatureBlock label="Firma">sizeOf&lt;T&gt;(): usize</SignatureBlock>

`sizeOf` devuelve la cantidad de bytes ocupada por un tipo primitivo. Se evalúa en tiempo de compilación.

<!-- @include: ../../../.fragments/modules/memory/size-of/sizeof.md -->

Acepta tipos enteros, tipos de punto flotante, `boolean` y `char`. Los valores gestionados, como `string` y arrays dinámicos, se rechazan.
