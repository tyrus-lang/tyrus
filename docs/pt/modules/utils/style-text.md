---
title: styleText
description: Estilo ANSI de texto pelo módulo nativo tyrus:utils.
---

# styleText

`styleText` envolve texto em sequências ANSI para saída de terminal.

<SignatureBlock label="Assinatura">styleText(format, text): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/utils/style-text/styletext.md -->
O compilador reconhece múltiplos formatos de chamada. Nomes de estilo devem ser string literals ou membros estáticos de estilo para que estilos inválidos sejam reportados em tempo de compilação.

::: warning Use stderr para chrome de terminal
Mensagens estilizadas de status normalmente devem ir para stderr. Mantenha stdout para saída normal de dados.
:::
