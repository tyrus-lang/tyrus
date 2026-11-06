---
title: Console / Print
description: Escrevendo em stdout e stderr com console.log, console.error, console.warn, print, printError, printWarn, printRaw e printCR.
---

# Console / Print

O Tyrus oferece duas formas equivalentes de escrever no terminal: o objeto `console`, e as funções ambientes `print`/`printError`/`printWarn`/`printRaw`/`printCR`, disponíveis sem nenhum import.

## console.log

`console.log` escreve em stdout. Todo argumento precisa ser imprimível: primitivos (`string`, os tipos numéricos, `boolean`, `char`), expressões regulares, arrays e tuplas de valores imprimíveis, e interfaces/objetos, que imprimem como `{ campo: valor, ... }`.

Múltiplos argumentos são unidos por um único espaço na mesma linha, com um `\n` final sempre incluído.

<!-- @include: ../../.fragments/language/console-print/console-log.md -->

## console.error

`console.error` escreve em stderr. Ela aceita os mesmos tipos de argumento que `console.log`, os une do mesmo jeito e acrescenta o mesmo `\n` final.

<!-- @include: ../../.fragments/language/console-print/console-error.md -->

## console.warn

`console.warn` escreve em stderr. Ela aceita os mesmos tipos de argumento que `console.log`, os une do mesmo jeito e acrescenta o mesmo `\n` final.

<!-- @include: ../../.fragments/language/console-print/console-warn.md -->

## Cor em `warn` e `error`

Em um terminal, `console.warn` colore a linha inteira de amarelo e `console.error` colore de vermelho, aberto antes do primeiro argumento e resetado depois do último, então os espaços entre argumentos também ficam coloridos:

<!-- @include: ../../.fragments/language/console-print/color.md -->

::: tip Mesma precedência do styleText
A decisão de colorir é tomada uma vez por processo a partir da detecção de terminal do próprio `stderr`. `NO_COLOR` (não-vazio) vence, depois `FORCE_COLOR`, senão detecção de TTY, a mesma precedência que [styleText](/pt/modules/utils/style-text) já documenta. `console.log`, `print` e `printRaw` nunca colorem.
:::

## print

`print` é uma função global ambiente, não precisa de import, do mesmo jeito que `assert` está sempre disponível. Ela se comporta exatamente como `console.log`: os mesmos tipos de argumento aceitos, os mesmos argumentos unidos por espaço, o mesmo `\n` final e o mesmo destino (stdout).

<!-- @include: ../../.fragments/language/console-print/print.md -->

::: tip print e console.log compartilham o mesmo caminho de código
`print` e `console.log` compilam para exatamente a mesma sequência de escritas. Nenhum dos dois é mais rápido que o outro, escolha o que ler melhor no ponto de chamada.
:::

## printError

`printError` é uma função global ambiente que se comporta exatamente como `console.error`: os mesmos tipos de argumento aceitos, os mesmos argumentos unidos por espaço, o mesmo `\n` final e o mesmo destino (stderr). Em um terminal, ela colore a linha inteira de vermelho, com a mesma precedência de cor que `console.error` usa.

<!-- @include: ../../.fragments/language/console-print/print-error.md -->

::: tip printError compartilha o caminho de console.error
`printError` compila para exatamente a mesma emissão que `console.error`. Use o que ler melhor no ponto de chamada.
:::

## printWarn

`printWarn` é uma função global ambiente que se comporta exatamente como `console.warn`: os mesmos tipos de argumento aceitos, os mesmos argumentos unidos por espaço, o mesmo `\n` final e o mesmo destino (stderr). Em um terminal, ela colore a linha inteira de amarelo, com a mesma precedência de cor que `console.warn` usa.

<!-- @include: ../../.fragments/language/console-print/print-warn.md -->

::: tip printWarn compartilha o caminho de console.warn
`printWarn` compila para exatamente a mesma emissão que `console.warn`. Use o que ler melhor no ponto de chamada.
:::

## printRaw

`printRaw` é `print` sem o `\n` final. Chamadas consecutivas caem na mesma linha do terminal, o que é útil para indicadores de progresso ou para montar a saída aos poucos:

<!-- @include: ../../.fragments/language/console-print/print-raw.md -->

::: tip printRaw dá flush explícito
`print`/`console.log` nunca precisam: eles sempre terminam em `\n`, e o line buffering do próprio SO já descarrega o buffer sozinho nisso. `printRaw` não termina em `\n`, então o tyrus dá flush explícito no stdout logo depois, sem isso, o texto poderia ficar parado no buffer, invisível, até algo mais acabar dando flush depois.
:::

## printCR

`printCR` é `printRaw` que também apaga o que a chamada *anterior* de `printCR` deixou na linha atual primeiro, útil pra barra de progresso, spinner ou qualquer saída que atualiza no lugar em vez de rolar:

<!-- @include: ../../.fragments/language/console-print/print-cr.md -->

::: tip Sem estado, e seguro fora de um terminal
Apagar funciona escrevendo `\r` (volta pra coluna 0) seguido do escape ANSI "limpar até o fim da linha" (`\x1B[K`), depois o texto novo, sem nenhum controle em runtime de quanto media a linha anterior. Essa sequência de escape é pulada inteiramente quando o stdout não é um terminal de verdade (um pipe, um log de CI): ela não limparia nada ali, só corromperia o log com bytes de escape crus, então o `printCR` degrada pra escrever o texto puro, igual `printRaw`, nesse caso.
:::

## Nomes reservados

`console`, `print`, `printError`, `printWarn`, `printRaw`, `printCR`, `assert`, `assertEqual` e `assertThrows` são reservados. Eles lexam como identificadores comuns, não como keywords da linguagem, mas uma declaração que reuse um desses nomes, função, variável, tipo, parâmetro, catch binding, alias de import ou nome desestruturado, é rejeitada (`E0003`), exatamente como uma keyword reservada de verdade, tipo `const` ou `throw`. Isso garante que a mesma chamada sempre signifique a mesma coisa, em todo arquivo:

::: danger error
<!-- @include: ../../.fragments/language/console-print/reserved-names.md -->
:::