---
title: alignOf
description: Alinhamento em bytes, em tempo de compilação, de um tipo primitivo.
---

# alignOf

<SignatureBlock label="Assinatura">alignOf&lt;T&gt;(): usize</SignatureBlock>

`alignOf` retorna a fronteira em bytes exigida para armazenar um tipo primitivo. Um `f64` tem alinhamento `8`, então seu endereço deve ser múltiplo de 8. O compilador insere padding dentro de structs quando necessário para satisfazer essa exigência. Ela é avaliada em tempo de compilação.

<!-- @include: ../../../.fragments/modules/memory/align-of/alignof.md -->

Ela aceita tipos inteiros, tipos de ponto flutuante, `boolean` e `char`. Valores gerenciados, como `string` e arrays dinâmicos, são rejeitados.
