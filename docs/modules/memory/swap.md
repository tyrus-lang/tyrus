---
title: swap
description: Exchange two mutable bindings of the same Copy type.
---

# swap

<SignatureBlock label="Signature">swap&lt;T&gt;(left: T, right: T): void</SignatureBlock>

`swap` exchanges the values of two distinct mutable local bindings of the same type. It supports `Copy` values and `string`; exchanging strings transfers their owned allocations between the two bindings without copying their contents.

<!-- @include: ../../.fragments/modules/memory/swap.md -->
