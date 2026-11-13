---
title: zeroize
description: Apaga e libera strings e bytes sensíveis.
---

# zeroize

<SignatureBlock label="Assinatura">zeroize(value: string): Result&lt;void, MemoryError&gt;</SignatureBlock>

<SignatureBlock label="Assinatura">zeroize(value: u8[]): Result&lt;void, MemoryError&gt;</SignatureBlock>

`zeroize(value)` consome uma `string` ou `u8[]` local. Ela sobrescreve a alocação com zeros por escritas voláteis, coloca uma barreira de compilador após as escritas e libera a alocação. O binding não pode ser usado depois.

Strings dinâmicas devem ter exatamente uma referência viva. Literais de string, empréstimos e strings compartilhadas são rejeitados com `MemoryError`, pois apagá-los escreveria armazenamento somente leitura ou alteraria dados ainda visíveis por outro valor.

<!-- @include: ../../../.fragments/modules/memory/zeroize/zeroize.md -->
