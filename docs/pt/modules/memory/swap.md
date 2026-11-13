---
title: swap
description: Troca dois bindings mutáveis do mesmo tipo Copy.
---

# swap

<SignatureBlock label="Assinatura">swap&lt;T&gt;(left: T, right: T): void</SignatureBlock>

`swap` troca os valores de dois bindings locais mutáveis distintos do mesmo tipo. Aceita valores `Copy` e `string`; trocar strings transfere suas alocações de posse entre os dois bindings sem copiar seu conteúdo.

<!-- @include: ../../../.fragments/modules/memory/swap.md -->
