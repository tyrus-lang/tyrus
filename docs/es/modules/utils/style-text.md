---
title: styleText
description: Estilo ANSI de texto mediante el módulo nativo tyrus:utils.
---

# styleText

`styleText` envuelve texto en secuencias ANSI para salida de terminal.

<SignatureBlock label="Firma">styleText(format, text): string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/utils/style-text/styletext.md -->
El compilador reconoce múltiples formas de llamada. Los nombres de estilo deben ser string literals o miembros estáticos de estilo para que estilos inválidos se reporten en tiempo de compilación.

::: warning Usa stderr para chrome de terminal
Los mensajes de status estilizados normalmente deben ir a stderr. Mantén stdout para salida normal de datos.
:::
