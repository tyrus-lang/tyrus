---
title: EOL
description: La constante nativa de fin de línea del módulo nativo tyrus:os.
---

# EOL

`EOL` es la secuencia nativa de fin de línea del sistema operativo: `"\r\n"` en Windows, `"\n"` en los demás.

<SignatureBlock label="Firma">EOL: string</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-eol.md -->
::: tip Una constante, no una llamada a función
A diferencia de cualquier otro export de `tyrus:os`, `EOL` es un valor simple, no una función, escribe `EOL`, no `EOL()`. Se resuelve a un valor fijo cuando el propio compilador fue construido, así que referenciarla nunca implica una consulta en tiempo de ejecución.
:::
