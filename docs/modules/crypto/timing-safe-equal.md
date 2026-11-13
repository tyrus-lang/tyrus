---
title: timingSafeEqual
description: Constant-time comparison for equal-length data through tyrus:crypto.
---

# timingSafeEqual

`timingSafeEqual` compares equal-length data in constant time to prevent timing side-channel attacks.

<SignatureBlock label="Signature">timingSafeEqual(a: string | u8[], b: string | u8[]): boolean</SignatureBlock>



<!-- @include: ../../.fragments/modules/crypto/timing-safe-equal/timingsafeequal.md -->
Different lengths return `false` immediately; the function does not conceal data length.
