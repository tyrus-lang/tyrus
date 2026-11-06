---
title: figlet
description: FIGlet banner rendering through the native tyrus:utils module.
---

# figlet

`figlet` renders text into FIGlet ASCII-art banners.

<SignatureBlock label="Signature">figlet(text: string, font?: "standard" | "small" | "big" | "slant"): string</SignatureBlock>



<!-- @include: ../../.fragments/modules/utils/figlet/figlet.md -->
The optional font must be a supported string literal. Unknown fonts and runtime font variables are reported by diagnostics.
