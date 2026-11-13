---
title: sleepAsync
description: Suspende uma task assíncrona sem bloquear sua thread do executor através de tyrus:task.
---

# sleepAsync

`sleepAsync` resolve após a quantidade solicitada de milissegundos sem bloquear uma thread do executor.

<SignatureBlock label="Assinatura">sleepAsync(ms: u64): Promise&lt;void&gt;</SignatureBlock>

Aguarde a Promise retornada dentro de uma função `async`. O código abaixo desse `await` na mesma função espera o atraso, enquanto outras tasks prontas podem continuar na thread do executor.

Para uma espera bloqueante em código síncrono, use [`sleep`](./sleep.md).

<!-- @include: ../../../.fragments/modules/task/sleep-async-example.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/task/sleep-async-error.md -->
:::
