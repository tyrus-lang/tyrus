---
title: sleepAsync
description: Suspende una task asíncrona sin bloquear su hilo del executor a través de tyrus:task.
---

# sleepAsync

`sleepAsync` se resuelve después de la cantidad solicitada de milisegundos sin bloquear un hilo del executor.

<SignatureBlock label="Firma">sleepAsync(ms: u64): Promise&lt;void&gt;</SignatureBlock>

Espere la Promise retornada dentro de una función `async`. El código debajo de ese `await` en la misma función espera el retraso, mientras otras tasks listas pueden continuar en el hilo del executor.

Para una espera bloqueante en código síncrono, use [`sleep`](./sleep.md).

<!-- @include: ../../../.fragments/modules/task/sleep-async-example.md -->

::: danger error
<!-- @include: ../../../.fragments/modules/task/sleep-async-error.md -->
:::
