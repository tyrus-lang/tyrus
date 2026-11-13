---
title: zeroize
description: Erase and release sensitive strings and bytes.
---

# zeroize

<SignatureBlock label="Signature">zeroize(value: string): Result&lt;void, MemoryError&gt;</SignatureBlock>

<SignatureBlock label="Signature">zeroize(value: u8[]): Result&lt;void, MemoryError&gt;</SignatureBlock>

`zeroize(value)` consumes a local `string` or `u8[]`. It overwrites the allocation with zeroes through volatile writes, places a compiler fence after the writes, and releases the allocation. The binding cannot be used afterward.

Dynamic strings must have exactly one live reference. String literals, borrows, and shared strings are rejected with `MemoryError` because erasing them would either write read-only storage or mutate data still visible through another value.

<!-- @include: ../../.fragments/modules/memory/zeroize/zeroize.md -->
