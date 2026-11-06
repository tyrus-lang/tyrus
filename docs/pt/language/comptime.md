---
title: Avaliação em Tempo de Compilação
description: Avalie expressões determinísticas e construa constantes durante a compilação com comptime.
---

# Avaliação em Tempo de Compilação

`comptime(expressão)` avalia uma expressão determinística durante a compilação e substitui o wrapper por um literal comum no programa gerado. Ele pode avaliar literals diretamente ou chamar uma `comptime function` ou arrow `comptime const` explicitamente elegível. A mesma função qualificada continua podendo ser chamada normalmente em runtime.

## Pré-calcule Fibonacci

Este exemplo calcula o número de Fibonacci na posição 50 durante a compilação. A implementação iterativa termina em 50 passos, enquanto uma versão recursiva ingênua repetiria trabalho exponencialmente e poderia exceder o orçamento determinístico de avaliação. O resultado usa `i64` porque não cabe em `i32`.

<!-- @include: ../../.fragments/language/comptime/fibonacci.md -->

O loop e as mutações intermediárias executam somente no compilador. Em runtime, `fibonacci50` já é o literal `12586269025`. Chamar `fibonacci(...)` fora de `comptime(...)` continua válido e executa normalmente em runtime.

## Funções Elegíveis

O alvo de uma chamada em tempo de compilação precisa usar `comptime function nome(...)` ou `comptime const nome = (...) => ...`. Código elegível pode usar bindings locais, condicionais, loops, recursão, `break`, `continue`, arrays, tuples, records, enums e aritmética determinística. Ele não pode ler estado externo mutável, chamar uma função comum ou nativa, fazer I/O, usar tempo ou aleatoriedade, lançar ou capturar erros, usar `async`/`await` nem criar um valor com identidade de runtime. Uma arrow de compilação pode capturar apenas constantes que também possam ser avaliadas durante a compilação.

## Valores e Limites

Resultados de compilação podem ser booleans, inteiros, valores de ponto flutuante suportados, decimals, chars, strings, `void`, arrays, tuples, records e enums cujo conteúdo também seja um valor de compilação. Closures, valores de função, referências, promises, regexes, `Map`, `Set` e recursos nativos são rejeitados. Cada wrapper possui limites determinísticos de 1.000.000 de passos de avaliação, 256 chamadas aninhadas, 65.536 elementos estruturais e 16 MiB de valores temporários e materializados.

## Feedback no Editor

Passe o cursor sobre a keyword `comptime` para ver o tipo avaliado e um resumo limitado do valor. Falhas apontam para a causa exata e incluem o wrapper e o rastro de chamadas como informações relacionadas. Quando uma expressão é válida em runtime, mas não é elegível para avaliação em compilação, o editor pode remover o wrapper com o Quick Fix `Evaluate this expression at runtime`.
