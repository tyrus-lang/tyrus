---
title: drop
description: Libera um valor com posse antes da saída do escopo.
---

# drop

<SignatureBlock label="Assinatura">drop(value: string): void</SignatureBlock>

<SignatureBlock label="Assinatura">drop&lt;T&gt;(value: T[]): void</SignatureBlock>

`drop(value)` consome uma `string`, array, coleção ou handle nativo local e libera seus recursos imediatamente. Valores estáticos não precisam ser liberados. O binding não pode ser usado depois.

Quando o fluxo de controle tem mais de um caminho possível, o binding só é considerado descartado após o desvio quando todos os caminhos que continuam o descartam. Um caminho que termina com `return` ou `throw` não afeta a disponibilidade de um caminho irmão que continua.

<!-- @include: ../../../.fragments/modules/memory/drop.md -->
