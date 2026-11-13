---
title: figlet
description: Renderização de banner FIGlet pelo módulo nativo tyrus:utils.
---

# figlet

`figlet` renderiza texto em banners de arte ASCII FIGlet.

<SignatureBlock label="Assinatura">figlet(text: string, font?: "standard" | "small" | "big" | "slant"): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/utils/figlet/figlet.md -->
A fonte opcional deve ser uma string literal suportada. Fontes desconhecidas e variáveis de fonte em runtime são reportadas por diagnósticos.
