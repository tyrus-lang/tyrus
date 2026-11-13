---
title: figlet
description: Renderización de banner FIGlet mediante el módulo nativo tyrus:utils.
---

# figlet

`figlet` renderiza texto en banners de arte ASCII FIGlet.

<SignatureBlock label="Firma">figlet(text: string, font?: "standard" | "small" | "big" | "slant"): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/utils/figlet/figlet.md -->
La fuente opcional debe ser una string literal soportada. Fuentes desconocidas y variables de fuente en runtime son reportadas por diagnósticos.
