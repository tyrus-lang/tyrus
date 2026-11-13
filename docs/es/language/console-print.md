---
title: Console / Print
description: Escribiendo en stdout y stderr con console.log, console.error, console.warn, print, printError, printWarn, printRaw y printCR.
---

# Console / Print

Tyrus ofrece dos formas equivalentes de escribir en la terminal: el objeto `console`, y las funciones ambiente `print`/`printError`/`printWarn`/`printRaw`/`printCR`, disponibles sin ningún import.

## console.log

`console.log` escribe en stdout. Todo argumento debe ser imprimible: primitivos (`string`, los tipos numéricos, `boolean`, `char`), expresiones regulares, arrays y tuplas de valores imprimibles, e interfaces/objetos, que se imprimen como `{ campo: valor, ... }`.

Varios argumentos se unen con un único espacio en la misma línea, con un `\n` final siempre incluido.

<!-- @include: ../../.fragments/language/console-print/console-log.md -->

## console.error

`console.error` escribe en stderr. Acepta los mismos tipos de argumento que `console.log`, los une del mismo modo y agrega el mismo `\n` final.

<!-- @include: ../../.fragments/language/console-print/console-error.md -->

## console.warn

`console.warn` escribe en stderr. Acepta los mismos tipos de argumento que `console.log`, los une del mismo modo y agrega el mismo `\n` final.

<!-- @include: ../../.fragments/language/console-print/console-warn.md -->

## Color en `warn` y `error`

En una terminal, `console.warn` colorea toda su línea de amarillo y `console.error` la colorea de rojo, abierto antes del primer argumento y reiniciado después del último, así que los espacios entre argumentos también quedan coloreados:

<!-- @include: ../../.fragments/language/console-print/color.md -->

::: tip La misma precedencia que styleText
La decisión de colorear se toma una vez por proceso a partir de la detección de terminal del propio `stderr`. `NO_COLOR` (no vacío) gana, luego `FORCE_COLOR`, si no detección de TTY, la misma precedencia que [styleText](/es/modules/utils/style-text) ya documenta. `console.log`, `print` y `printRaw` nunca colorean.
:::

## print

`print` es una función global ambiente, no requiere import, igual que `assert` siempre está disponible. Se comporta exactamente igual que `console.log`: los mismos tipos de argumento aceptados, los mismos argumentos unidos por espacio, el mismo `\n` final y el mismo destino (stdout).

<!-- @include: ../../.fragments/language/console-print/print.md -->

::: tip print y console.log comparten el mismo camino de código
`print` y `console.log` compilan exactamente a la misma secuencia de escrituras. Ninguno es más rápido que el otro, elige el que se lea mejor en el punto de llamada.
:::

## printError

`printError` es una función global ambiente que se comporta exactamente igual que `console.error`: los mismos tipos de argumento aceptados, los mismos argumentos unidos por espacio, el mismo `\n` final y el mismo destino (stderr). En una terminal, colorea toda su línea de rojo, con la misma precedencia de color que usa `console.error`.

<!-- @include: ../../.fragments/language/console-print/print-error.md -->

::: tip printError comparte el camino de console.error
`printError` compila exactamente a la misma emisión que `console.error`. Usa el que se lea mejor en el punto de llamada.
:::

## printWarn

`printWarn` es una función global ambiente que se comporta exactamente igual que `console.warn`: los mismos tipos de argumento aceptados, los mismos argumentos unidos por espacio, el mismo `\n` final y el mismo destino (stderr). En una terminal, colorea toda su línea de amarillo, con la misma precedencia de color que usa `console.warn`.

<!-- @include: ../../.fragments/language/console-print/print-warn.md -->

::: tip printWarn comparte el camino de console.warn
`printWarn` compila exactamente a la misma emisión que `console.warn`. Usa el que se lea mejor en el punto de llamada.
:::

## printRaw

`printRaw` es `print` sin el `\n` final. Llamadas consecutivas caen en la misma línea de la terminal, lo cual es útil para indicadores de progreso o para construir la salida poco a poco:

<!-- @include: ../../.fragments/language/console-print/print-raw.md -->

::: tip printRaw hace flush explícito
`print`/`console.log` nunca lo necesitan: siempre terminan en `\n`, y el line buffering del propio SO ya descarga el buffer por su cuenta con eso. `printRaw` no termina en `\n`, así que tyrus hace flush explícito de stdout justo después, sin eso, el texto podría quedarse en el buffer, invisible, hasta que algo más terminara haciendo flush después.
:::

## printCR

`printCR` es `printRaw` que además borra lo que la llamada *anterior* a `printCR` dejó en la línea actual primero, útil para barras de progreso, spinners, o cualquier salida que se actualiza en el lugar en vez de desplazarse:

<!-- @include: ../../.fragments/language/console-print/print-cr.md -->

::: tip Sin estado, y seguro fuera de una terminal
Borrar funciona escribiendo `\r` (vuelve a la columna 0) seguido del escape ANSI "limpiar hasta el final de línea" (`\x1B[K`), luego el texto nuevo, sin ningún control en runtime de cuánto medía la línea anterior. Esa secuencia de escape se omite por completo cuando stdout no es una terminal real (un pipe, un log de CI): no limpiaría nada ahí, solo corrompería el log con bytes de escape crudos, así que `printCR` degrada a escribir su texto sin más, como `printRaw`, en ese caso.
:::

## Nombres reservados

`console`, `print`, `printError`, `printWarn`, `printRaw`, `printCR`, `assert`, `assertEqual` y `assertThrows` son reservados. Lexan como identificadores comunes, no como keywords del lenguaje, pero una declaración que reutilice uno de estos nombres, función, variable, tipo, parámetro, catch binding, alias de import o nombre desestructurado, es rechazada (`E0003`), exactamente igual que una keyword reservada de verdad, como `const` o `throw`. Esto garantiza que la misma llamada siempre signifique lo mismo, en todo archivo:

::: danger error
<!-- @include: ../../.fragments/language/console-print/reserved-names.md -->
:::