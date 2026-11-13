---
title: replace
description: Store a Copy value and return the previous value.
---

# replace

<SignatureBlock label="Signature">replace&lt;T&gt;(destination: T, source: T): T</SignatureBlock>

`replace` stores `source` in a mutable local binding and returns the value that was previously stored there. It supports `Copy` types and `string`. For `string`, both arguments must be mutable local bindings; `source` is moved and cannot be used after the call.

<!-- @include: ../../.fragments/modules/memory/replace.md -->

::: danger error
<!-- @include: ../../.fragments/modules/memory/replace-error.md -->
:::
