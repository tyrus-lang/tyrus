---
title: sleep
description: Bloqueia a thread atual através de tyrus:task.
---

# sleep

`sleep` bloqueia a thread atual pela quantidade solicitada de milissegundos.

<SignatureBlock label="Assinatura">sleep(ms: u64): void</SignatureBlock>

Use `sleep` em programas de linha de comando, scripts simples e testes nos quais bloquear a thread atual é intencional. Em um programa assíncrono com uma única thread de trabalho, ele bloqueia todo o runtime.

## O que continua durante a espera

O código abaixo de `await sleepAsync(...)` na mesma função espera o atraso terminar. Outras tasks prontas podem usar a thread enquanto a função atual está suspensa. No primeiro programa, `sleep` bloqueia a thread, então imprime `first` e depois `second`. No segundo programa, `sleepAsync` suspende a primeira task, então imprime `second` e depois `first`.

<!-- @include: ../../../.fragments/modules/task/sleep-ordering.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/task/sleep-error.md -->
:::
