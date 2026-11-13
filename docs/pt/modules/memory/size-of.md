---
title: sizeOf
description: Tamanho em bytes, em tempo de compilação, de um tipo primitivo.
---

# sizeOf

<SignatureBlock label="Assinatura">sizeOf&lt;T&gt;(): usize</SignatureBlock>

`sizeOf` retorna a quantidade de bytes ocupada por um tipo primitivo. Ela é avaliada em tempo de compilação.

<!-- @include: ../../../.fragments/modules/memory/size-of/sizeof.md -->

Ela aceita tipos inteiros, tipos de ponto flutuante, `boolean` e `char`. Valores gerenciados, como `string` e arrays dinâmicos, são rejeitados.
