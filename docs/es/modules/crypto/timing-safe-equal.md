---
title: timingSafeEqual
description: Comparación en tiempo constante para datos del mismo tamaño a través de tyrus:crypto.
---

# timingSafeEqual

`timingSafeEqual` compara datos del mismo tamaño en tiempo constante para prevenir ataques de side-channel por timing.

<SignatureBlock label="Firma">timingSafeEqual(a: string | u8[], b: string | u8[]): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/crypto/timing-safe-equal/timingsafeequal.md -->
Los tamaños diferentes devuelven `false` inmediatamente; la función no oculta el tamaño de los datos.
