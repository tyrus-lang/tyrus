---
title: drop
description: Release an owned value before scope exit.
---

# drop

<SignatureBlock label="Signature">drop(value: string): void</SignatureBlock>

<SignatureBlock label="Signature">drop&lt;T&gt;(value: T[]): void</SignatureBlock>

`drop(value)` consumes a local `string`, array, collection, or native handle and releases its resources immediately. Static values need no release. The binding cannot be used afterward.

When control flow has more than one possible path, the binding is considered dropped after the branch only when every path that continues drops it. A path that ends with `return` or `throw` does not affect the availability of a sibling path that continues.

<!-- @include: ../../.fragments/modules/memory/drop.md -->
