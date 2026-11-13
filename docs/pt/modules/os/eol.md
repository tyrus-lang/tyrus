---
title: EOL
description: A constante nativa de fim de linha do módulo nativo tyrus:os.
---

# EOL

`EOL` é a sequência nativa de fim de linha do sistema operacional: `"\r\n"` no Windows, `"\n"` nos demais.

<SignatureBlock label="Assinatura">EOL: string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-eol.md -->
::: tip Uma constante, não uma chamada de função
Ao contrário de todo outro export de `tyrus:os`, `EOL` é um valor simples, não uma função, escreva `EOL`, não `EOL()`. Ela resolve para um valor fixo quando o próprio compilador foi construído, então referenciá-la nunca envolve uma consulta em tempo de execução.
:::
