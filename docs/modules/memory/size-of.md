---
title: sizeOf
description: Compile-time size in bytes of a primitive type.
---

# sizeOf

<SignatureBlock label="Signature">sizeOf&lt;T&gt;(): usize</SignatureBlock>

`sizeOf` returns the number of bytes occupied by a primitive type. It is evaluated at compile time.

<!-- @include: ../../.fragments/modules/memory/size-of/sizeof.md -->

It accepts integer types, floating-point types, `boolean`, and `char`. Managed values such as `string` and dynamic arrays are rejected.
