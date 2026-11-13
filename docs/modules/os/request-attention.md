---
title: requestAttention
description: Ring the terminal bell to request the user's attention through the native tyrus:os module.
---

# requestAttention

`requestAttention` rings the terminal bell (`BEL`, `\x07`) on stderr, Tyrus programs are CLI/server binaries with no GUI window of their own to flash a taskbar icon on, so the bell is the closest cross-platform equivalent.

<SignatureBlock label="Signature">requestAttention(critical: boolean): void</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-request-attention.md -->
`critical` rings the bell three times instead of once.
