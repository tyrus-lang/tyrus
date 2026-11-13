---
title: timingSafeEqual
description: Comparação em tempo constante para dados de mesmo tamanho através de tyrus:crypto.
---

# timingSafeEqual

`timingSafeEqual` compara dados de mesmo tamanho em tempo constante para prevenir ataques de side-channel por timing.

<SignatureBlock label="Assinatura">timingSafeEqual(a: string | u8[], b: string | u8[]): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/crypto/timing-safe-equal/timingsafeequal.md -->
Tamanhos diferentes retornam `false` imediatamente; a função não oculta o tamanho dos dados.
