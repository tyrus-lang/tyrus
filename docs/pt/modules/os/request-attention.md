---
title: requestAttention
description: Toque o bell do terminal para chamar a atenção do usuário através do módulo nativo tyrus:os.
---

# requestAttention

`requestAttention` toca o bell do terminal (`BEL`, `\x07`) no stderr, programas Tyrus são binários de CLI/servidor sem janela GUI própria pra piscar um ícone na barra de tarefas, então o bell é o equivalente cross-platform mais próximo.

<SignatureBlock label="Assinatura">requestAttention(critical: boolean): void</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-request-attention.md -->
`critical` toca o bell três vezes em vez de uma.
