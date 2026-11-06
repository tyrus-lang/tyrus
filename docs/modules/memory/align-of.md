---
title: alignOf
description: Compile-time alignment in bytes of a primitive type.
---

# alignOf

<SignatureBlock label="Signature">alignOf&lt;T&gt;(): usize</SignatureBlock>

`alignOf` returns the byte boundary required to store a primitive type. An `f64` has alignment `8`, so its address must be a multiple of 8. The compiler inserts padding inside structs when needed to satisfy this requirement. It is evaluated at compile time.

<!-- @include: ../../.fragments/modules/memory/align-of/alignof.md -->

It accepts integer types, floating-point types, `boolean`, and `char`. Managed values such as `string` and dynamic arrays are rejected.
