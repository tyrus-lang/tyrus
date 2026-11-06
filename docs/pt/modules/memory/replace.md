---
title: replace
description: Armazena um valor Copy e retorna o valor anterior.
---

# replace

<SignatureBlock label="Assinatura">replace&lt;T&gt;(destination: T, source: T): T</SignatureBlock>

`replace` armazena `source` em um binding local mutável e retorna o valor antes armazenado nele. Aceita tipos `Copy` e `string`. Para `string`, os dois argumentos devem ser bindings locais mutáveis; `source` é movido e não pode ser usado após a chamada.

<!-- @include: ../../../.fragments/modules/memory/replace.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/memory/replace-error.md -->
:::
