---
title: requestAttention
description: Suena el bell de la terminal para pedir la atención del usuario mediante el módulo nativo tyrus:os.
---

# requestAttention

`requestAttention` suena el bell de la terminal (`BEL`, `\x07`) en stderr, los programas Tyrus son binarios de CLI/servidor sin ventana GUI propia para hacer parpadear un ícono en la barra de tareas, así que el bell es el equivalente cross-platform más cercano.

<SignatureBlock label="Firma">requestAttention(critical: boolean): void</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-request-attention.md -->
`critical` suena el bell tres veces en vez de una.
